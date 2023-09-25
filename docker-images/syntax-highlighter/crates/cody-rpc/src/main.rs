use std::{
    collections::HashMap,
    path::Path,
    time::{Duration, Instant},
};

use gix::{bstr::ByteSlice, objs::tree::EntryMode, traverse::tree::Recorder};
use scip_syntax::{languages::get_tag_configuration, symbols::parse_tree};
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

fn find_all_lens_git() -> StatsMap {
    // let mut n = 0;
    let mut map = StatsMap::new();

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

    for a in recorder.records {
        if a.mode == EntryMode::Blob {
            // if a.filepath.ends_with(".ts".as_bytes()) {
            let now = Instant::now();
            let bundled_parser = if let Some(parser) =
                BundledParser::get_parser_from_extension(if let Ok(path) = a.filepath.to_str() {
                    Path::new(path)
                        .extension()
                        .unwrap_or(a.filepath.to_os_str().expect("a"))
                        .to_str()
                        .expect("abc")
                } else {
                    continue;
                }) {
                parser
            } else {
                continue;
            };

            parser
                .set_language(bundled_parser.get_language())
                .expect("abc");

            let a = &repo.find_object(a.oid).expect("a").data;
            let source = if let Ok(str) = a.to_str() {
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

            let nanos = now.elapsed().as_nanos();
            // n +=
            let entry = map.entry(bundled_parser).or_insert_with(|| LangStats {
                symbols: 0,
                lines: 0,
                nanos: 0,
            });
            entry.lines += source.lines().count();
            entry.symbols += scope.into_document(hint, vec![]).occurrences.len();
            entry.nanos += nanos;
            // println!("{:#?}", );
            // }
        }
    }
    return map;
}
