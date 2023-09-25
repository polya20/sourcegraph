use std::{path::Path, time::Instant};

use gix::{bstr::ByteSlice, objs::tree::EntryMode, traverse::tree::Recorder};
use scip_syntax::{languages::get_tag_configuration, symbols::parse_tree};
use scip_treesitter_languages::parsers::BundledParser;

fn main() {
    let now = Instant::now();
    let n = find_all_lens_git();
    println!("{}", now.elapsed().as_millis());
    println!("{}", n);
}

fn find_all_lens_git() -> usize {
    let mut n = 0;
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

            n += scope.into_document(hint, vec![]).occurrences.len()
            // println!("{:#?}", );
            // }
        }
    }
    return n;
}
