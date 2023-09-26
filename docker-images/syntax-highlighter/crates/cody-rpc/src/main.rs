use std::{
    collections::HashMap,
    path::Path,
    time::{Duration, Instant},
};

use gix::{bstr::ByteSlice, objs::tree::EntryMode, traverse::tree::Recorder};
use scip::types::Document;
use scip_syntax::{globals::parse_tree, languages::get_tag_configuration};
use scip_treesitter_languages::parsers::BundledParser;
use tabled::{Table, Tabled};

fn main() {
    let n = find_all_lens_git();

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

type Oid = [u8; 20];
type OidToDocument = HashMap<Oid, Document>;

fn find_all_lens_git() -> StatsMap {
    // let mut n = 0;
    let mut map = StatsMap::new();
    let mut resolve_symbol_by_lang = HashMap::<BundledParser, OidToDocument>::new();

    let repo = gix::open("/Users/auguste.rame/Documents/Repos/sourcegraph/.git").expect("bruh");

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

            let entry = resolve_symbol_by_lang
                .entry(bundled_parser.clone())
                .or_insert(OidToDocument::new());

            let gix::ObjectId::Sha1(oid) = record.oid;
            entry
                .entry(oid)
                .or_insert(scope.into_document(hint, vec![]));

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
    return map;
}
