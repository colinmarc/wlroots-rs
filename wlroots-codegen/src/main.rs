use anyhow::{Context, Result};
use clap::Parser;
use lazy_static::lazy_static;
use regex::Regex;
use std::{collections::HashMap, path::PathBuf};
use tree_sitter::{Node, Query, QueryCapture, QueryCursor, QueryMatch};

#[derive(clap::Parser, Debug)]
#[command(author, version, about)]
struct Args {
    path: PathBuf, // File to read
}

#[derive(Debug)]
enum CType {
    Void,
    Value(String),
    Enum(String),
    StructPointer(String),
    Unknown(String),
}

#[derive(Debug)]
struct WlrMethod {
    doc: Option<String>,
    name: String,
    params: Vec<CParam>,
    return_type: CType,
}

#[derive(Debug)]
struct CParam {
    name: String,
    ty: CType,
}

#[derive(Debug)]
struct WlrStruct {
    doc: Option<String>,
    name: String,
    events: Vec<WlrEvent>,
}

#[derive(Debug)]
struct WlrEvent {
    doc: Option<String>,
    name: String,
    event_type: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let source = std::fs::read_to_string(args.path)?;
    let source = source.as_bytes();
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(tree_sitter_c::language())
        .context("failed to load C grammar")?;

    let tree = parser.parse(&source, None).unwrap();

    let methods = parse_methods(tree.root_node(), &source)?;
    let structs = parse_structs(tree.root_node(), &source)?;

    for st in structs.iter() {
        eprintln!("parsed struct: {:#?}", st);
    }

    for m in methods.iter() {
        eprintln!("parsed method: {:#?}", m);
    }

    Ok(())
}

fn parse_methods(node: Node, source_bytes: &[u8]) -> Result<Vec<WlrMethod>> {
    lazy_static! {
        static ref METHOD_QUERY: Query = Query::new(
            tree_sitter_c::language(),
            r#"
            (
                (comment)* @doc .
                (declaration
                    type: (_) @return_type
                    declarator: [
                        (function_declarator
                            declarator: (identifier) @name
                            parameters: (_) @params
                        )
                        (pointer_declarator
                            declarator: (function_declarator
                                declarator: (identifier) @name
                                parameters: (_) @params
                            )
                        ) @pointer_type
                    ]
                ) @wlr_method
                (#select-adjacent! @doc @wlr_method)
                (#match? @name "^wlr_[a-z_]+")
            )
            "#
        )
        .expect("error building method query");
    };

    let mut methods = Vec::new();
    for m in QueryCursor::new().matches(&METHOD_QUERY, node, &source_bytes[..]) {
        let captures = captures_by_name(m, &METHOD_QUERY);

        let doc = match captures.get("doc") {
            Some(c) => Some(extract_comment_text(c.node.utf8_text(&source_bytes)?)),
            None => None,
        };

        let name = captures
            .get("name")
            .unwrap()
            .node
            .utf8_text(&source_bytes)?
            .to_string();

        let mut ret = captures
            .get("return_type")
            .unwrap()
            .node
            .utf8_text(&source_bytes)?;

        let params = parse_method_params(captures.get("params").unwrap().node, source_bytes)?;
        let return_type = parse_type(ret, captures.get("pointer_type").is_some());

        methods.push(WlrMethod {
            doc,
            name,
            params,
            return_type,
        })
    }

    Ok(methods)
}

fn parse_method_params(node: Node, source_bytes: &[u8]) -> Result<Vec<CParam>> {
    lazy_static! {
        static ref PARAM_QUERY: Query = Query::new(
            tree_sitter_c::language(),
            r#"
            (parameter_declaration
                type: (_) @param_type
                declarator: [
                    (pointer_declarator
                        declarator: (identifier) @param_name
                    ) @pointer_type
                    (identifier) @param_name
                ]
            )
        "#
        )
        .expect("error building param query");
    };

    let mut params = Vec::new();
    for m in QueryCursor::new().matches(&PARAM_QUERY, node, &source_bytes[..]) {
        let captures = captures_by_name(m, &PARAM_QUERY);

        let name = captures
            .get("param_name")
            .unwrap()
            .node
            .utf8_text(&source_bytes)?
            .to_string();

        let mut ty = captures
            .get("param_type")
            .unwrap()
            .node
            .utf8_text(&source_bytes)?;

        let ty = parse_type(ty, captures.get("pointer_type").is_some());
        params.push(CParam { name, ty })
    }

    Ok(params)
}

fn parse_structs(node: Node, source_bytes: &[u8]) -> Result<Vec<WlrStruct>> {
    lazy_static! {
        static ref STRUCT_QUERY: Query = Query::new(
            tree_sitter_c::language(),
            r#"
            (
                (comment)* @doc .
                (struct_specifier
                    name: (type_identifier) @name
                        (#match? @name "^wlr_[a-z_]+")
                    body: (field_declaration_list
                        (field_declaration
                            type: (struct_specifier)
                            declarator: (field_identifier) @declarator
                                (#match? @declarator "^events$")
                        )? @events
                    )
                ) @wlr_struct
                (#select-adjacent! @doc @wlr_struct)
            )
            "#
        )
        .expect("error building struct query");
    };

    let mut structs = Vec::new();
    for m in QueryCursor::new().matches(&STRUCT_QUERY, node, &source_bytes[..]) {
        let captures = captures_by_name(m, &STRUCT_QUERY);
        let events = match captures.get("events") {
            Some(c) => parse_events_decl(c.node, &source_bytes)?,
            None => Vec::new(),
        };

        let doc = match captures.get("doc") {
            Some(c) => Some(extract_comment_text(c.node.utf8_text(&source_bytes)?)),
            None => None,
        };

        let name = captures
            .get("name")
            .unwrap()
            .node
            .utf8_text(&source_bytes)?
            .to_string();

        structs.push(WlrStruct { doc, name, events })
    }

    Ok(structs)
}

fn parse_events_decl(node: Node, source_bytes: &[u8]) -> Result<Vec<WlrEvent>> {
    lazy_static! {
        // TODO: the below parser fails to pick up multi-line "//"-style
        // comments correctly. I'm really not sure why.
        static ref EVENTS_QUERY: Query = Query::new(
            tree_sitter_c::language(),
            r#"
            (
                (comment)* @doc .
                (field_declaration
                    type: (struct_specifier
                        name: (type_identifier) @field_type)
                    declarator: (field_identifier) @event_name
                        (#match? @field_type "^wl_signal$")
                ) @decl
                .
                (comment)? @event_type
                (#select-adjacent! @doc @decl)
                (#select-adjacent! @event_type @decl)
            )
        "#
        )
        .expect("error building events query");
        static ref EVENT_ANNOTATION_RE: Regex =
            Regex::new(r"\A\/\/ (struct )?(?P<type>wlr_[a-z_]+)(\\\*)?\z").unwrap();
    };

    let mut events = Vec::new();
    for event_match in QueryCursor::new().matches(&EVENTS_QUERY, node, source_bytes) {
        let captures = captures_by_name(event_match, &EVENTS_QUERY);

        let name = captures
            .get("event_name")
            .unwrap()
            .node
            .utf8_text(&source_bytes)?
            .to_string();

        let doc = match captures.get("doc") {
            Some(c) => {
                let text = c.node.utf8_text(&source_bytes)?.to_string();

                // Sometimes we parse the type annotation for the previous
                // line as documentation.
                if EVENT_ANNOTATION_RE.is_match(&text) {
                    None
                } else {
                    Some(extract_comment_text(&text))
                }
            }
            _ => None,
        };

        let event_type = match captures.get("event_type") {
            Some(c) => {
                let text = c.node.utf8_text(&source_bytes)?.to_string();
                EVENT_ANNOTATION_RE
                    .captures(&text)
                    .map(|c| c.name("type").unwrap().as_str().to_string())
            }
            _ => None,
        };

        events.push(WlrEvent {
            name,
            event_type,
            doc,
        });
    }

    Ok(events)
}

fn parse_type(s: &str, is_pointer: bool) -> CType {
    if s.starts_with("enum ") && !is_pointer {
        CType::Enum(s.trim_start_matches("enum ").to_string())
    } else if s.starts_with("struct ") && is_pointer {
        CType::StructPointer(s.trim_start_matches("struct ").to_string())
    } else if s == "void" && !is_pointer {
        CType::Void
    } else if !is_pointer {
        CType::Value(s.to_string())
    } else {
        CType::Unknown(s.to_string())
    }
}

fn extract_comment_text(s: &str) -> String {
    lazy_static! {
        static ref COMMENT_RE: Regex = Regex::new(r"\A\s*[\/\*]+\s+(?P<text>.*)\s*").unwrap();
    }

    let comment = s
        .lines()
        .flat_map(|l| {
            COMMENT_RE
                .captures(l)
                .map(|c| c.name("text").unwrap().as_str().to_string())
        })
        .collect::<Vec<String>>()
        .join(" ");

    if !comment.ends_with(".") {
        comment + "."
    } else {
        comment
    }
}

fn captures_by_name<'a>(
    m: QueryMatch<'a, '_>,
    query: &Query,
) -> HashMap<String, &'a QueryCapture<'a>> {
    m.captures
        .iter()
        .map(|c: &QueryCapture| (query.capture_names()[c.index as usize].clone(), c))
        .collect()
}
