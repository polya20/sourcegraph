use std::collections::{HashMap, HashSet};

use gix::bstr::ByteSlice;
use gix::Repository;
use scip::types::Document;
use scip_macros::include_scip_query;
use scip_treesitter::{types::PackedRange, NodeToScipRange};
use scip_treesitter_languages::parsers::BundledParser;

use crate::types::{self, SymbolContextSnippet};

pub type Oid = [u8; 20];
pub type OidToDocument = HashMap<Oid, Document>;
pub type OidSet = HashSet<Oid>;
pub type NameToOids = HashMap<String, OidSet>;
pub type LangAndNameToOids = HashMap<BundledParser, NameToOids>;

pub struct Index {
    pub oid_to_document: OidToDocument,
    pub lang_and_name_to_oids: LangAndNameToOids,
}

pub fn symbol_snippets_near_cursor(
    repo: &Repository,
    index: &Index,

    bundled_parser: BundledParser,
    content: String,
    position: types::Position,

    depth: u8,
    max_depth: u8,
) -> Result<Vec<SymbolContextSnippet>, ()> {
    let mut snippets = vec![];

    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(bundled_parser.get_language())
        .expect("abc");

    let query = tree_sitter::Query::new(
        parser.language().expect("bruh"),
        include_scip_query!("typescript", "context"),
    )
    .expect("bruh");

    let capture_names = query.capture_names();

    let source_bytes = content.as_bytes();
    let tree = parser.parse(source_bytes, None).expect("bruh");

    let mut cursor = tree_sitter::QueryCursor::new();
    let cursor_range = PackedRange {
        start_line: position.line as i32,
        start_col: position.character as i32,
        end_line: position.line as i32,
        end_col: position.character as i32,
    };

    for m in cursor.matches(&query, tree.root_node(), source_bytes) {
        let mut identifier = None;

        for capture in m.captures {
            let capture_name = capture_names
                .get(capture.index as usize)
                .expect("capture indexes should always work");

            match capture_name.as_str() {
                "identifier" => {
                    identifier = Some(capture.node);
                }
                "range" => {
                    if !PackedRange::from_vec(&capture.node.to_scip_range())
                        .unwrap()
                        .contains(&cursor_range)
                    {
                        continue;
                    }
                }
                &_ => {}
            }
        }

        match identifier {
            Some(identifier) => snippets.append(
                &mut symbol_snippets_from_identifier(
                    &repo,
                    index,
                    identifier.utf8_text(source_bytes).unwrap().to_string(),
                    0,
                    4,
                )
                .unwrap(),
            ),
            None => {}
        }
    }

    Ok(snippets)
}

pub fn symbol_snippets_from_identifier(
    repo: &Repository,
    index: &Index,

    identifier: String,

    depth: u8,
    max_depth: u8,
) -> Result<Vec<SymbolContextSnippet>, ()> {
    let mut snippets = vec![];

    if depth >= max_depth {
        return Ok(snippets);
    }

    let oids = match index
        .lang_and_name_to_oids
        .get(&BundledParser::Typescript)
        .expect("no lang bundle")
        .get(&identifier)
    {
        Some(identifier) => identifier,
        None => return Ok(snippets),
    };

    for oid in oids {
        let document = index.oid_to_document.get(oid).expect("no document");
        for occu in &document.occurrences {
            let data = &repo.find_object(*oid).expect("no oid").data;
            let source = if let Ok(source) = data.to_str() {
                source
            } else {
                continue;
            };

            if scip::symbol::parse_symbol(occu.symbol.as_str())
                .unwrap()
                .descriptors
                .last()
                .unwrap()
                .name
                == identifier
            {
                if occu.enclosing_range.len() != 0 {
                    let range = PackedRange::from_vec(&occu.enclosing_range)
                        .expect("no vec range")
                        .to_range(&source)
                        .expect("No range");

                    snippets.push(SymbolContextSnippet {
                        file_name: document.relative_path.clone(),
                        symbol: occu.symbol.clone(),
                        content: source[range].to_string(),
                    });

                    snippets.append(
                        &mut find_related_symbol_snippets(
                            repo,
                            index,
                            source.to_string(),
                            PackedRange::from_vec(&occu.range).expect("no vec range"),
                            depth + 1,
                            max_depth,
                        )
                        .expect("failed to find related symbol snippets"),
                    );
                }
            }
        }
    }

    Ok(snippets)
}

pub fn find_related_symbol_snippets(
    repo: &Repository,
    index: &Index,

    content: String,
    identifier_range: PackedRange,

    depth: u8,
    max_depth: u8,
) -> Result<Vec<SymbolContextSnippet>, ()> {
    let mut snippets = vec![];

    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(BundledParser::Typescript.get_language())
        .expect("abc");

    let query = tree_sitter::Query::new(
        parser.language().expect("bruh"),
        include_scip_query!("typescript", "context"),
    )
    .expect("bruh");

    let capture_names = query.capture_names();

    let source_bytes = content.as_bytes();
    let tree = parser.parse(source_bytes, None).expect("bruh");

    let mut cursor = tree_sitter::QueryCursor::new();

    for m in cursor.matches(&query, tree.root_node(), source_bytes) {
        let mut name = None;
        let mut related = vec![];

        for capture in m.captures {
            let capture_name = capture_names
                .get(capture.index as usize)
                .expect("capture indexes should always work");

            match capture_name.as_str() {
                "name" => {
                    name = Some(capture.node);
                }
                "related" => {
                    related.push(capture.node);
                }
                &_ => {}
            }
        }

        match name {
            Some(name) => {
                if PackedRange::from_vec(&name.to_scip_range()).expect("no vec range")
                    == identifier_range
                {
                    for related in related {
                        eprint!("{}", related.utf8_text(source_bytes).unwrap().to_string());
                        snippets.append(&mut symbol_snippets_from_identifier(
                            repo,
                            index,
                            related.utf8_text(source_bytes).unwrap().to_string(),
                            depth,
                            max_depth,
                        )?)
                    }
                }
            }
            None => {}
        }
    }

    Ok(snippets)
}
