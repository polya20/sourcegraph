use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use context::{DocumentContext, SymbolToSymbolInformation};
use gix::{bstr::ByteSlice, objs::tree::EntryMode, traverse::tree::Recorder, Repository};
use scip_syntax::{globals::parse_tree, languages::get_tag_configuration};
use scip_treesitter_languages::parsers::BundledParser;
use std::error::Error;

use crate::types::SymbolContextSnippet;
use lsp_server::{Connection, ExtractError, Message, Response};

mod context;
mod types;

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    let (connection, _) = Connection::stdio();

    main_loop(connection)?;

    eprintln!("shutting down server");
    Ok(())
}

fn main_loop(connection: Connection) -> Result<(), Box<dyn Error + Sync + Send>> {
    eprintln!("starting example main loop");

    let mut git_dir: Option<PathBuf> = None;
    let mut indices = RepoIndices::new();

    for msg in &connection.receiver {
        // eprintln!("got msg: {msg:?}");
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                // eprintln!("got request: {req:?}");

                match req.method.as_str() {
                    "bfg/initialize" => {
                        match types::cast_request::<types::Initialize>(req) {
                            Ok((id, _)) => {
                                let result = Some(types::InitializeResponse {
                                    server_version: env!("CARGO_PKG_VERSION").to_string(),
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
                            Err(ExtractError::MethodMismatch(_)) => panic!("bruh"),
                        };
                    }
                    "bfg/contextAtPosition" => {
                        match types::cast_request::<types::ContextAtPosition>(req) {
                            Ok((id, params)) => {
                                let git_dir = match git_dir.clone() {
                                    Some(dir) => dir,
                                    None => {
                                        let response = Some(types::ContextAtPositionResponse {
                                            symbols: vec![],
                                            files: vec![],
                                        });
                                        let json_result = serde_json::to_value(&response).unwrap();
                                        let resp = Response {
                                            id,
                                            result: Some(json_result),
                                            error: None,
                                        };
                                        connection.sender.send(Message::Response(resp))?;
                                        continue;
                                    }
                                };

                                let repo = gix::open(&git_dir).expect("bruh");
                                let index = match indices.get(&git_dir) {
                                    Some(index) => index,
                                    None => {
                                        panic!("repo not found {:?} {:?}", &git_dir, indices.keys())
                                    }
                                };

                                let mut symbol_snippets: HashSet<SymbolContextSnippet> =
                                    HashSet::new();

                                context::symbol_snippets_near_cursor(
                                    &mut symbol_snippets,
                                    &repo,
                                    index,
                                    BundledParser::Typescript,
                                    params.content,
                                    params.position,
                                    0,
                                    4,
                                )
                                .expect("bruh");

                                let response = Some(types::ContextAtPositionResponse {
                                    symbols: symbol_snippets.into_iter().collect(),
                                    files: vec![],
                                });
                                let result = serde_json::to_value(&response).unwrap();
                                let resp = Response {
                                    id,
                                    result: Some(result),
                                    error: None,
                                };
                                connection.sender.send(Message::Response(resp))?;
                                continue;
                            }
                            Err(err @ ExtractError::JsonError { .. }) => panic!("{err:?}"),
                            Err(ExtractError::MethodMismatch(_)) => panic!("bruh"),
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
                                let index = index_repo(&repo).expect("not to fail");

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

fn index_repo<'a>(repo: &Repository) -> Result<context::Index, ()> {
    let mut index = context::Index {
        oid_to_document_context: context::OidToDocumentContext::new(),
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
            if !record.filepath.ends_with(".ts".as_bytes()) {
                continue;
            }

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
            let mut symbol_to_symbol_information = SymbolToSymbolInformation::new();

            let entry = index
                .lang_and_name_to_oids
                .entry(bundled_parser.clone())
                .or_insert(context::NameToOids::new());

            for (i, symbol) in document.symbols.iter().enumerate() {
                symbol_to_symbol_information.insert(symbol.symbol.clone(), i as u32);
            }

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

            index.oid_to_document_context.insert(
                oid,
                DocumentContext {
                    document,
                    symbol_to_symbol_information,
                },
            );
        }
    }

    return Ok(index);
}

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
        let index = index_repo(&repo).expect("not to fail");

        let oids = index
            .lang_and_name_to_oids
            .get(&BundledParser::Typescript)
            .unwrap()
            .get("sayHello")
            .unwrap();

        for oid in oids {
            for occu in &index
                .oid_to_document_context
                .get(oid)
                .unwrap()
                .document
                .occurrences
            {
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
