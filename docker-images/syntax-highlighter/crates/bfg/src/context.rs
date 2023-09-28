use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Result};
use gix::bstr::ByteSlice;
use gix::Repository;
use scip::types::Document;
use scip_macros::include_scip_query;
use scip_treesitter::{types::PackedRange, NodeToScipRange};
use scip_treesitter_languages::parsers::BundledParser;

use crate::types::{self, SymbolContextSnippet};

pub type Oid = [u8; 20];
pub type OidToDocumentContext = HashMap<Oid, DocumentContext>;
pub type OidSet = HashSet<Oid>;
pub type NameToOids = HashMap<String, OidSet>;
pub type LangAndNameToOids = HashMap<BundledParser, NameToOids>;
pub type SymbolToSymbolInformation = HashMap<String, u32>;

pub struct DocumentContext {
    pub document: Document,
    pub symbol_to_symbol_information: SymbolToSymbolInformation,
}

pub struct Index {
    pub oid_to_document_context: OidToDocumentContext,
    pub lang_and_name_to_oids: LangAndNameToOids,
    pub oid_to_basename: HashMap<Oid, String>,
}

pub fn symbol_snippets_near_cursor(
    snippets: &mut HashSet<SymbolContextSnippet>,
    repo: &Repository,
    index: &Index,

    bundled_parser: BundledParser,
    content: String,
    position: types::Position,

    depth: u8,
    max_depth: u8,
) -> Result<()> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(bundled_parser.get_language())?;

    let query = tree_sitter::Query::new(
        match parser.language() {
            Some(lang) => lang,
            None => return Err(anyhow!("Invalid tree-sitter language")),
        },
        include_scip_query!("typescript", "context"),
    )?;

    let capture_names = query.capture_names();

    let source_bytes = content.as_bytes();
    let tree = match parser.parse(source_bytes, None) {
        Some(tree) => tree,
        None => return Err(anyhow!("Failed to parse code")),
    };

    let mut cursor = tree_sitter::QueryCursor::new();
    let cursor_range = PackedRange {
        start_line: position.line as i32,
        start_col: position.character as i32,
        end_line: position.line as i32,
        end_col: position.character as i32,
    };

    'matches: for m in cursor.matches(&query, tree.root_node(), source_bytes) {
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
                        continue 'matches;
                    }
                }
                &_ => {}
            }
        }

        match identifier {
            Some(identifier) => symbol_snippets_from_identifier(
                snippets,
                &repo,
                index,
                identifier.utf8_text(source_bytes).unwrap().to_string(),
                depth,
                max_depth,
            )?,
            None => {}
        }
    }

    Ok(())
}

pub fn symbol_snippets_from_identifier(
    snippets: &mut HashSet<SymbolContextSnippet>,
    repo: &Repository,
    index: &Index,

    identifier: String,

    depth: u8,
    max_depth: u8,
) -> Result<()> {
    if depth >= max_depth {
        return Ok(());
    }

    let oids = match index
        .lang_and_name_to_oids
        .get(&BundledParser::Typescript)
        .expect("no lang bundle")
        .get(&identifier)
    {
        Some(identifier) => identifier,
        None => return Ok(()),
    };

    for oid in oids {
        let doc_context = index.oid_to_document_context.get(oid).expect("no document");
        let document = &doc_context.document;

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
                    let sig_doc = match doc_context.symbol_to_symbol_information.get(&occu.symbol) {
                        Some(si_index) => {
                            let si = &document.symbols[*si_index as usize];
                            eprintln!("{:?}", si);

                            let sig_doc = si.signature_documentation.as_ref();
                            match sig_doc {
                                Some(sig_doc) => Some(&sig_doc.text),
                                None => None,
                            }
                        }
                        None => None,
                    };

                    let content = match sig_doc {
                        Some(content) => content.clone(),
                        None => {
                            let range = PackedRange::from_vec(&occu.enclosing_range)
                                .expect("no vec range")
                                .to_range(&source)
                                .expect("No range");

                            source[range].to_string()
                        }
                    };

                    snippets.insert(SymbolContextSnippet {
                        file_name: index.oid_to_basename.get(oid).unwrap().to_string(),
                        symbol: occu.symbol.clone(),
                        content,
                    });

                    find_related_symbol_snippets(
                        snippets,
                        repo,
                        index,
                        source.to_string(),
                        PackedRange::from_vec(&occu.range).expect("no vec range"),
                        depth + 1,
                        max_depth,
                    )?;
                }
            }
        }
    }

    Ok(())
}

pub fn find_related_symbol_snippets(
    snippets: &mut HashSet<SymbolContextSnippet>,
    repo: &Repository,
    index: &Index,

    content: String,
    identifier_range: PackedRange,

    depth: u8,
    max_depth: u8,
) -> Result<()> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(BundledParser::Typescript.get_language())?;

    let query = tree_sitter::Query::new(
        match parser.language() {
            Some(lang) => lang,
            None => return Err(anyhow!("Invalid tree-sitter language")),
        },
        include_scip_query!("typescript", "context"),
    )?;

    let capture_names = query.capture_names();

    let source_bytes = content.as_bytes();
    let tree = match parser.parse(source_bytes, None) {
        Some(tree) => tree,
        None => return Err(anyhow!("Failed to parse code")),
    };

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
                        symbol_snippets_from_identifier(
                            snippets,
                            repo,
                            index,
                            related.utf8_text(source_bytes).unwrap().to_string(),
                            depth,
                            max_depth,
                        )?
                    }
                }
            }
            None => {}
        }
    }

    Ok(())
}
