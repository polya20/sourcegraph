use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use gix::{bstr::ByteSlice, objs::tree::EntryMode, traverse::tree::Recorder, Repository};
use scip::types::Document;
use scip_syntax::{globals::parse_tree, languages::get_tag_configuration};
use scip_treesitter::types::PackedRange;
use scip_treesitter_languages::parsers::BundledParser;
use std::error::Error;
use tabled::{Table, Tabled};
// use lsp_types::{ClientCapabilities, InitializeParams, ServerCapabilities};

use lsp_server::{Connection, ExtractError, Message, Notification, Request, RequestId, Response};
use serde::{Deserialize, Serialize};

mod context;
mod types;

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    let (connection, io_threads) = Connection::stdio();

    main_loop(connection)?;

    eprintln!("shutting down server");
    Ok(())
}

fn main_loop(connection: Connection) -> Result<(), Box<dyn Error + Sync + Send>> {
    eprintln!("starting example main loop");

    let mut git_dir: Option<PathBuf> = None;
    let mut indices = RepoIndices::new();

    for msg in &connection.receiver {
        eprintln!("got msg: {msg:?}");
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                eprintln!("got request: {req:?}");

                match req.method.as_str() {
                    "bfg/initialize" => {
                        match types::cast_request::<types::Initialize>(req) {
                            Ok((id, params)) => {
                                let result = Some(types::InitializeResponse {
                                    server_version: "pogchamp".to_string(),
                                });
                                let result = serde_json::to_value(&result).unwrap();
                                let resp = Response {
                                    id,
                                    result: Some(result),
                                    error: None,
                                };
                                connection.sender.send(Message::Response(resp))?;
                                continue;
                            }
                            Err(err @ ExtractError::JsonError { .. }) => panic!("{err:?}"),
                            Err(ExtractError::MethodMismatch(req)) => panic!("bruh"),
                        };
                    }
                    "bfg/contextAtPosition" => {
                        match types::cast_request::<types::ContextAtPosition>(req) {
                            Ok((id, params)) => {
                                let git_dir = git_dir.clone().expect("no git dir");

                                let repo = gix::open(&git_dir).expect("bruh");
                                let index = match indices.get(&git_dir) {
                                    Some(index) => index,
                                    None => {
                                        panic!("repo not found {:?} {:?}", &git_dir, indices.keys())
                                    }
                                };

                                let mut symbol_snippets = vec![];
                                let identifiers = context::get_identifiers_near_cursor(
                                    BundledParser::Typescript,
                                    params.content,
                                    params.position,
                                )
                                .expect("bruh");

                                for identifier in identifiers {
                                    symbol_snippets.append(
                                        &mut context::symbol_snippets_from_identifier(
                                            &repo, index, identifier, 0, 4,
                                        )
                                        .unwrap(),
                                    )
                                }

                                let result = Some(types::ContextAtPositionResponse {
                                    symbols: symbol_snippets,
                                    files: vec![],
                                });
                                let result = serde_json::to_value(&result).unwrap();
                                let resp = Response {
                                    id,
                                    result: Some(result),
                                    error: None,
                                };
                                connection.sender.send(Message::Response(resp))?;
                                continue;
                            }
                            Err(err @ ExtractError::JsonError { .. }) => panic!("{err:?}"),
                            Err(ExtractError::MethodMismatch(req)) => panic!("bruh"),
                        };
                    }
                    "bfg/gitRevision/didChange" => {
                        match types::cast_request::<types::GitRevisionDidChange>(req) {
                            Ok((id, params)) => {
                                let new_git_dir = url::Url::parse(&params.git_directory_uri)
                                    .expect("bruh")
                                    .to_file_path()
                                    .expect("bruh");

                                let repo = gix::open(&new_git_dir).expect("bruh");
                                let (_, index) = index_repo(&repo).expect("not to fail");

                                indices.insert(new_git_dir.to_path_buf(), index);

                                git_dir = Some(new_git_dir);

                                let result = Some(());
                                let result = serde_json::to_value(&result).unwrap();
                                let resp = Response {
                                    id,
                                    result: Some(result),
                                    error: None,
                                };
                                connection.sender.send(Message::Response(resp))?;
                                continue;
                            }
                            Err(err @ ExtractError::JsonError { .. }) => panic!("{err:?}"),
                            Err(ExtractError::MethodMismatch(req)) => req,
                        };
                    }
                    &_ => {}
                }
            }
            Message::Response(resp) => {
                eprintln!("got response: {resp:?}");
            }
            Message::Notification(not) => {
                eprintln!("got notification: {not:?}");
            }
        }
    }
    Ok(())
}

type RepoIndices = HashMap<PathBuf, context::Index>;

fn foo() {
    let repo = gix::open("/Users/auguste.rame/Documents/Repos/sourcegraph/.git").expect("bruh");
    let (n, lang_map) = index_repo(&repo).expect("not to fail");

    let mut entries = vec![];

    for (k, v) in &n {
        let secs = Duration::from_nanos(v.nanos as u64).as_secs_f64();

        entries.push(TableEntry {
            language: k.get_language_name().to_string(),
            symbols: v.symbols,
            lines: v.lines,
            symbols_per_second: ((v.symbols as f64) / (secs)) as usize,
            lines_per_second: ((v.lines as f64) / (secs)) as usize,
        });
    }

    println!("{}", Table::new(entries).to_string());
}

#[derive(Tabled)]
struct TableEntry {
    language: String,
    symbols: usize,
    lines: usize,

    symbols_per_second: usize,
    lines_per_second: usize,
}

#[derive(Debug)]
struct LangStats {
    symbols: usize,
    lines: usize,
    nanos: u128,
}

type StatsMap = HashMap<BundledParser, LangStats>;

fn index_repo<'a>(repo: &Repository) -> Result<(StatsMap, context::Index), ()> {
    let mut map = StatsMap::new();
    let mut index = context::Index {
        oid_to_document: context::OidToDocument::new(),
        lang_and_name_to_oids: context::LangAndNameToOids::new(),
    };

    let tree = repo
        .rev_parse_single("HEAD")
        .expect("a")
        .object()
        .expect("azbc")
        .peel_to_tree()
        .expect("a");
    let mut recorder = Recorder::default();
    tree.traverse().breadthfirst(&mut recorder).expect("abc");

    let mut parser = tree_sitter::Parser::new();

    for record in recorder.records {
        if record.mode == EntryMode::Blob {
            // if a.filepath.ends_with(".ts".as_bytes()) {
            let now = Instant::now();
            let bundled_parser = if let Some(parser) = BundledParser::get_parser_from_extension(
                if let Ok(path) = record.filepath.to_str() {
                    Path::new(path)
                        .extension()
                        .unwrap_or(record.filepath.to_os_str().expect("a"))
                        .to_str()
                        .expect("abc")
                } else {
                    continue;
                },
            ) {
                parser
            } else {
                continue;
            };

            parser
                .set_language(bundled_parser.get_language())
                .expect("abc");

            let data = &repo.find_object(record.oid).expect("a").data;
            let source = if let Ok(str) = data.to_str() {
                str
            } else {
                continue;
            };

            let tree = parser.parse(source, None).expect("tree");

            let (mut scope, hint) = parse_tree(
                if let Some(config) = get_tag_configuration(&bundled_parser) {
                    config
                } else {
                    continue;
                },
                &tree,
                source.as_bytes(),
            )
            .expect("a");

            let gix::ObjectId::Sha1(oid) = record.oid;
            let document = scope.into_document(hint, vec![]);

            let entry = index
                .lang_and_name_to_oids
                .entry(bundled_parser.clone())
                .or_insert(context::NameToOids::new());

            for occu in &document.occurrences {
                let symbol = match scip::symbol::parse_symbol(occu.symbol.as_str()) {
                    Ok(symbol) => symbol,
                    Err(_) => continue,
                };
                let entry = entry
                    .entry(
                        symbol
                            .descriptors
                            .last()
                            .clone()
                            .expect("non-empty symbol")
                            .name
                            .clone(),
                    )
                    .or_insert(context::OidSet::new());
                entry.insert(oid);
            }

            index.oid_to_document.insert(oid, document);

            let nanos = now.elapsed().as_nanos();
            let entry = map.entry(bundled_parser.clone()).or_insert(LangStats {
                symbols: 0,
                lines: 0,
                nanos: 0,
            });
            entry.lines += source.lines().count();
            entry.symbols += scope.into_document(hint, vec![]).occurrences.len();
            entry.nanos += nanos;
        }
    }

    return Ok((map, index));
}

// fn query(oid: lang_map: &LangMap, language: BundledParser) -> Vec<Symbol> {
//     let symbols = vec![];

//     match lang_map.get(language) {
//         Some(oid_to_document) => {},
//         None => return symbols,
//     }

//     symbols
// }

#[cfg(test)]
mod test {
    use super::*;
    use std::path::Path;
    use std::process::Command;

    use gix;
    use gix::Repository;
    use scip_treesitter::types::PackedRange;

    fn load_repo(path: &Path) -> Repository {
        let _ = std::fs::remove_dir_all(path.join(Path::new(".git")));

        Command::new("git")
            .args(["init"])
            .current_dir(path)
            .output()
            .expect("failed to execute process");

        Command::new("git")
            .args(["add", "."])
            .current_dir(path)
            .output()
            .expect("failed to execute process");

        Command::new("git")
            .args(["commit", "--no-gpg-sign", "-m", "init"])
            .current_dir(path)
            .output()
            .expect("failed to execute process");

        gix::open(path).expect("Could not init repo!")
    }

    fn delete_git(path: &Path) {
        let _ = std::fs::remove_dir_all(path.join(Path::new(".git")));
    }

    #[test]
    fn test_typescript() {
        let git_dir = Path::new("testdata/typescript");
        let repo = load_repo(&git_dir);
        let (stats, index) = index_repo(&repo).expect("not to fail");

        let oids = index
            .lang_and_name_to_oids
            .get(&BundledParser::Typescript)
            .unwrap()
            .get("sayHello")
            .unwrap();

        for oid in oids {
            for occu in &index.oid_to_document.get(oid).unwrap().occurrences {
                let data = &repo.find_object(*oid).unwrap().data;
                let source = if let Ok(str) = data.to_str() {
                    str
                } else {
                    continue;
                };

                if occu.enclosing_range.len() != 0 {
                    let range = PackedRange::from_vec(&occu.enclosing_range)
                        .unwrap()
                        .to_range(&source)
                        .expect("No range");

                    println!("a: {:?} ", &source[range]);
                }
            }
        }

        delete_git(git_dir);
    }
}
