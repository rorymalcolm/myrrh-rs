use anyhow::{Context, Result};
use clap::Parser;
use itertools::Itertools;
use serde_json::Value;
use std::collections::HashSet;
use tracing::{event, span, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser, Debug)]
struct Args {
    #[clap(short = 'i', long = "input", value_parser)]
    input_file: String,

    #[clap(short = 'o', long = "output", value_parser)]
    output_file: Option<String>,
}

#[derive(Debug)]
enum TypeScriptPrimativeType {
    String,
    Boolean,
    Number,
    Object,
    Array,
    Null,
}

#[derive(Debug)]
struct TypeScriptNode {
    name: Option<String>,
    nullable: bool,
    optional: bool,
    is_array: bool,
    root_node: bool,
    sub_items: Vec<TypeScriptNode>,
    type_signature: TypeScriptPrimativeType,
}

impl TypeScriptNode {
    fn new(
        type_name: TypeScriptPrimativeType,
        optional: bool,
        nullable: bool,
        is_array: bool,
        root_node: bool,
    ) -> Self {
        TypeScriptNode {
            name: None,
            nullable,
            optional,
            is_array,
            root_node,
            sub_items: Vec::new(),
            type_signature: type_name,
        }
    }

    fn with_sub_items(mut self, sub_items: Vec<TypeScriptNode>) -> Self {
        self.sub_items = sub_items;
        self
    }

    fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    fn to_type_string(node: TypeScriptNode, array_node: bool) -> String {
        let mut type_string = String::new();
        type_string.push_str("type DefaultType = ");
        type_string.push_str(&Self::to_type_string_helper(node, array_node, 0));
        type_string
    }

    fn newline_if_parent_not_array_node(array_node: bool) -> String {
        if !array_node {
            String::from("\n")
        } else {
            String::new()
        }
    }

    fn semicolon_if_parent_array_node(array_node: bool) -> String {
        if array_node {
            String::from(";")
        } else {
            String::new()
        }
    }

    fn space_if_parent_not_root_node(root_node: bool) -> String {
        if !root_node {
            String::from(" ")
        } else {
            String::new()
        }
    }

    fn string_is_alphanumeric(string: &str) -> bool {
        string.chars().all(|c| c.is_alphanumeric() || c == '_')
    }

    fn to_type_string_helper(
        node: TypeScriptNode,
        parent_array_node: bool,
        indent_size: usize,
    ) -> String {
        let mut type_string = String::new();
        let mut indent_string = String::new();
        if !node.root_node && !parent_array_node {
            for _ in 0..indent_size {
                indent_string.push_str("  ");
            }
            type_string.push_str(&indent_string)
        }
        match node.name {
            Some(name) => {
                if Self::string_is_alphanumeric(&name.clone()) {
                    type_string.push_str(&format!("{}: ", name));
                } else {
                    type_string.push_str(&format!("\"{}\": ", name))
                }
            }
            None => (),
        }
        match node.type_signature {
            TypeScriptPrimativeType::Boolean => type_string.push_str("boolean"),
            TypeScriptPrimativeType::String => type_string.push_str("string"),
            TypeScriptPrimativeType::Number => type_string.push_str("number"),
            TypeScriptPrimativeType::Null => type_string.push_str("null"),
            TypeScriptPrimativeType::Object => {
                let mut object_type_string = String::new();
                object_type_string.push_str(&format!(
                    "{{{}{}",
                    &Self::newline_if_parent_not_array_node(parent_array_node),
                    &Self::space_if_parent_not_root_node(node.root_node)
                ));
                for o in node.sub_items {
                    object_type_string.push_str(&format!(
                        "{}{}{}",
                        TypeScriptNode::to_type_string_helper(
                            o,
                            parent_array_node,
                            indent_size + 1,
                        ),
                        &Self::space_if_parent_not_root_node(parent_array_node),
                        &Self::semicolon_if_parent_array_node(parent_array_node)
                    ));
                }
                object_type_string.push_str(&format!(
                    "{}{}}}",
                    &Self::space_if_parent_not_root_node(node.root_node),
                    indent_string
                ));
                type_string.push_str(&object_type_string.clone())
            }
            TypeScriptPrimativeType::Array => {
                let mut array_types_seen = HashSet::<String>::new();
                for a in node.sub_items {
                    let array_type =
                        TypeScriptNode::to_type_string_helper(a, true, indent_size + 1);
                    array_types_seen.insert(array_type);
                }
                if array_types_seen.len() == 0 {
                    type_string.push_str("any");
                } else if array_types_seen.len() == 1 {
                    type_string.push_str(&format!("{}", array_types_seen.iter().next().unwrap()));
                } else {
                    type_string.push_str(&format!(
                        "({})",
                        &array_types_seen.iter().sorted().join(" | ")
                    ));
                }
                type_string.push_str("[]");
            }
        }
        if node.optional {
            type_string.push_str("?");
        }
        if node.nullable {
            type_string.push_str("null");
        }
        if !parent_array_node {
            type_string.push_str(";\n");
        }
        type_string
    }
}

fn walk_value_tree(v: &Value, key_name: Option<String>) -> Result<TypeScriptNode> {
    walk_value_tree_helper(v, key_name, true)
}

fn walk_value_tree_helper(
    v: &Value,
    key_name: Option<String>,
    root_node: bool,
) -> Result<TypeScriptNode> {
    match v {
        Value::String(_s) => {
            let mut node = TypeScriptNode::new(
                TypeScriptPrimativeType::String,
                false,
                false,
                false,
                root_node,
            );
            if let Some(name) = key_name {
                node = node.with_name(name);
            }
            Ok(node)
        }
        Value::Number(_n) => {
            let mut node = TypeScriptNode::new(
                TypeScriptPrimativeType::Number,
                false,
                false,
                false,
                root_node,
            );
            if let Some(name) = key_name {
                node = node.with_name(name);
            }
            Ok(node)
        }
        Value::Bool(_b) => {
            let mut node = TypeScriptNode::new(
                TypeScriptPrimativeType::Boolean,
                false,
                false,
                false,
                root_node,
            );
            if let Some(name) = key_name {
                node = node.with_name(name);
            }
            Ok(node)
        }
        Value::Null => {
            let mut node = TypeScriptNode::new(
                TypeScriptPrimativeType::Null,
                false,
                false,
                false,
                root_node,
            );
            if let Some(name) = key_name {
                node = node.with_name(name);
            }
            Ok(node)
        }
        Value::Array(a) => {
            let mut node = TypeScriptNode::new(
                TypeScriptPrimativeType::Array,
                false,
                false,
                true,
                root_node,
            );
            let mut sub_items = Vec::new();
            for v in a {
                sub_items.push(walk_value_tree_helper(v, None, false)?);
            }
            if let Some(name) = key_name {
                node = node.with_name(name);
            }

            node = node.with_sub_items(sub_items);
            Ok(node)
        }
        Value::Object(o) => {
            let mut node = TypeScriptNode::new(
                TypeScriptPrimativeType::Object,
                false,
                false,
                false,
                root_node,
            );
            let mut sub_items = Vec::new();
            for (k, v) in o {
                sub_items.push(walk_value_tree_helper(
                    v,
                    Option::Some(k.to_string()),
                    false,
                )?);
            }
            if let Some(name) = key_name {
                node = node.with_name(name);
            }
            node = node.with_sub_items(sub_items);
            Ok(node)
        }
    }
}
fn main() -> Result<()> {
    let subscrber = FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscrber).expect("setting tracing default failed");

    let span = span!(Level::INFO, "parsing");

    let _enter = span.enter();

    let args = Args::parse();
    let input_file_content = std::fs::read_to_string(&args.input_file)
        .with_context(|| format!("could not read file `{}`", &args.input_file))?;

    let input_length = String::len(&input_file_content);
    event!(
        Level::INFO,
        input_file_content_length = input_length,
        "input file content"
    );

    let v: Value = serde_json::from_str(input_file_content.as_str())
        .with_context(|| format!("could not parse json"))?;

    let result = walk_value_tree(&v, None).unwrap();
    let result_root_is_array = result.is_array.clone();
    let output_string = TypeScriptNode::to_type_string(result, result_root_is_array);
    if args.output_file.is_none() {
        event!(
            Level::INFO,
            output_string = output_string,
            "generated output"
        );
    } else {
        event!(
            Level::INFO,
            output_file = args.output_file.clone().unwrap(),
            "writing output to file"
        );
        std::fs::write(args.output_file.unwrap(), output_string)
            .with_context(|| format!("could not write to file"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{walk_value_tree, TypeScriptNode};

    #[test]
    fn parses_string() {
        let val_tree = serde_json::from_str(r#""hello""#).unwrap();
        let result = walk_value_tree(&val_tree, None).unwrap();
        let output_string = TypeScriptNode::to_type_string(result, false);
        assert_eq!(output_string, "type DefaultType = string;\n");
    }

    #[test]
    fn semi_complex_arrays() {
        let val_tree = serde_json::from_str(
            r#"{
            "woah lol": {
              "test": ["woah"],
              "test2": ["woaher", { "test": "example" }]
            }
          }
          "#,
        )
        .unwrap();
        let result = walk_value_tree(&val_tree, None).unwrap();
        let output_string = TypeScriptNode::to_type_string(result, false);
        assert_eq!(
            output_string,
            "type DefaultType = {\n  \"woah lol\": {\n     test: string[];\n     test2: (string | { test: string; })[];\n    };\n };\n".to_string()
        );
    }

    #[test]
    fn parses_number() {
        let val_tree = serde_json::from_str(r#"1"#).unwrap();
        let result = walk_value_tree(&val_tree, None).unwrap();
        let output_string = TypeScriptNode::to_type_string(result, false);
        assert_eq!(output_string, "type DefaultType = number;\n");
    }

    #[test]
    fn parses_bool() {
        let val_tree = serde_json::from_str(r#"true"#).unwrap();
        let result = walk_value_tree(&val_tree, None).unwrap();
        let output_string = TypeScriptNode::to_type_string(result, false);
        assert_eq!(output_string, "type DefaultType = boolean;\n");
    }

    #[test]
    fn parses_null() {
        let val_tree = serde_json::from_str(r#"null"#).unwrap();
        let result = walk_value_tree(&val_tree, None).unwrap();
        let output_string = TypeScriptNode::to_type_string(result, false);
        assert_eq!(output_string, "type DefaultType = null;\n");
    }

    #[test]
    fn parses_object() {
        let val_tree = serde_json::from_str(r#"{}"#).unwrap();
        let result = walk_value_tree(&val_tree, None).unwrap();
        let output_string = TypeScriptNode::to_type_string(result, false);
        assert_eq!(output_string, "type DefaultType = {\n};\n");
    }

    #[test]
    fn parses_array() {
        let val_tree = serde_json::from_str(r#"[]"#).unwrap();
        let result = walk_value_tree(&val_tree, None).unwrap();
        let output_string = TypeScriptNode::to_type_string(result, false);
        assert_eq!(output_string, "type DefaultType = any[];\n");
    }

    #[test]
    fn parses_object_with_array() {
        let val_tree = serde_json::from_str(r#"{ "test": [] }"#).unwrap();
        let result = walk_value_tree(&val_tree, None).unwrap();
        let output_string = TypeScriptNode::to_type_string(result, false);
        assert_eq!(
            output_string,
            "type DefaultType = {\n  test: any[];\n };\n"
        );
    }

    #[test]
    fn parses_object_with_object() {
        let val_tree = serde_json::from_str(r#"{ "test": { "test": "test" } }"#).unwrap();
        let result = walk_value_tree(&val_tree, None).unwrap();
        let output_string = TypeScriptNode::to_type_string(result, false);
        assert_eq!(
            output_string,
            "type DefaultType = {\n  test: {\n     test: string;\n    };\n };\n"
        );
    }

    #[test]
    fn parses_object_with_array_of_objects() {
        let val_tree =
            serde_json::from_str(r#"{ "test": [{ "test": "test" }, { "test": "test" }] }"#)
                .unwrap();
        let result = walk_value_tree(&val_tree, None).unwrap();
        let output_string = TypeScriptNode::to_type_string(result, false);
        assert_eq!(
            output_string,
            "type DefaultType = {\n  test: { test: string; }[];\n };\n"
        );
    }

    #[test]
    fn parses_object_with_array_of_arrays() {
        let val_tree = serde_json::from_str(r#"{ "test": [[], []] }"#).unwrap();
        let result = walk_value_tree(&val_tree, None).unwrap();
        let output_string = TypeScriptNode::to_type_string(result, false);
        assert_eq!(
            output_string,
            "type DefaultType = {\n  test: any[][];\n };\n"
        );
    }
}
