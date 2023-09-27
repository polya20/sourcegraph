use scip_macros::include_scip_query;
use scip_treesitter::{types::PackedRange, NodeToScipRange};
use scip_treesitter_languages::parsers::BundledParser;
use tree_sitter::LanguageError;

use crate::types;

pub fn get_symbols(
    bundled_parser: BundledParser,
    content: String,
    position: types::Position,
) -> Result<Vec<String>, ()> {
    let mut symbols = Vec::new();

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

    let tree = parser.parse(content.as_bytes(), None).expect("bruh");

    eprintln!("pogchamp content {:?}", content);

    eprintln!("pogchamp aaa");

    let mut cursor = tree_sitter::QueryCursor::new();
    let cursor_range = PackedRange {
        start_line: position.line as i32,
        start_col: position.character as i32,
        end_line: position.line as i32,
        end_col: position.character as i32,
    };

    for m in cursor.matches(&query, tree.root_node(), content.as_bytes()) {
        // let node = m.node;
        for capture in m.captures {
            let capture_name = capture_names
                .get(capture.index as usize)
                .expect("capture indexes should always work");

            let mut identifier = None;

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

            match identifier {
                Some(identifier) => {
                    symbols.push(content[identifier.byte_range()].to_string());
                }
                None => panic!("literally impossible"),
            }
        }
    }

    Ok(symbols)
}
