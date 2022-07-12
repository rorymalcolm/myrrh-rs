mod type_output_cache_entry;
pub mod typescript_node;

pub(crate) use anyhow::{Context, Result};
use clap::Parser;
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tracing::{event, span, Level};
use tracing_subscriber::FmtSubscriber;
use typescript_node::{TypeScriptNode, TypeScriptPrimativeType};

#[derive(Parser, Debug)]
struct Args {
    #[clap(short = 'i', long = "input", value_parser)]
    input_file: String,

    #[clap(short = 'o', long = "output", value_parser)]
    output_file: Option<String>,

    #[clap(short = 's', long = "squash", value_parser)]
    squash_common_types: Option<bool>,
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
    let mut result: TypeScriptNode = walk_value_tree(&v, None).unwrap();
    let result_root_is_array = result.is_array().clone();
    match args.squash_common_types {
        Some(val) => {
            if val {
                result.calculate_hash();
            }
        }
        None => {
            result.calculate_hash();
            ()
        }
    }
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

fn walk_value_tree(v: &Value, key_name: Option<String>) -> Result<TypeScriptNode> {
    let lookup_table = HashMap::<u64, usize>::new();
    walk_value_tree_helper(v, key_name, true, Arc::new(Mutex::new(lookup_table)))
}

fn walk_value_tree_helper(
    v: &Value,
    key_name: Option<String>,
    root_node: bool,
    lookup_table: Arc<Mutex<HashMap<u64, usize>>>,
) -> Result<TypeScriptNode> {
    match v {
        Value::String(_s) => {
            let mut node = TypeScriptNode::new(
                lookup_table.clone(),
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
                lookup_table.clone(),
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
                lookup_table.clone(),
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
                lookup_table.clone(),
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
                lookup_table.clone(),
                TypeScriptPrimativeType::Array,
                false,
                false,
                true,
                root_node,
            );
            let mut sub_items = Vec::new();
            for v in a {
                sub_items.push(walk_value_tree_helper(
                    v,
                    None,
                    false,
                    lookup_table.clone(),
                )?);
            }
            if let Some(name) = key_name {
                node = node.with_name(name);
            }

            node = node.with_sub_items(sub_items);
            Ok(node)
        }
        Value::Object(o) => {
            let mut node = TypeScriptNode::new(
                lookup_table.clone(),
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
                    lookup_table.clone(),
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

#[cfg(test)]
mod tests {
    use crate::{walk_value_tree, TypeScriptNode};

    #[test]
    fn parses_string() {
        let val_tree = serde_json::from_str(r#""hello""#).unwrap();
        let mut result = walk_value_tree(&val_tree, None).unwrap();
        result.calculate_hash();
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
        let mut result = walk_value_tree(&val_tree, None).unwrap();
        result.calculate_hash();
        let output_string = TypeScriptNode::to_type_string(result, false);
        assert_eq!(
            output_string,
            "type DefaultType = {\n  \"woah lol\": {\n     test: string[];\n     test2: (string | { test: string; })[];\n    };\n };\n".to_string()
        );
    }

    #[test]
    fn parses_number() {
        let val_tree = serde_json::from_str(r#"1"#).unwrap();
        let mut result = walk_value_tree(&val_tree, None).unwrap();
        result.calculate_hash();
        let output_string = TypeScriptNode::to_type_string(result, false);
        assert_eq!(output_string, "type DefaultType = number;\n");
    }

    #[test]
    fn parses_bool() {
        let val_tree = serde_json::from_str(r#"true"#).unwrap();
        let mut result = walk_value_tree(&val_tree, None).unwrap();
        result.calculate_hash();
        let output_string = TypeScriptNode::to_type_string(result, false);
        assert_eq!(output_string, "type DefaultType = boolean;\n");
    }

    #[test]
    fn parses_null() {
        let val_tree = serde_json::from_str(r#"null"#).unwrap();
        let mut result = walk_value_tree(&val_tree, None).unwrap();
        result.calculate_hash();
        let output_string = TypeScriptNode::to_type_string(result, false);
        assert_eq!(output_string, "type DefaultType = null;\n");
    }

    #[test]
    fn parses_object() {
        let val_tree = serde_json::from_str(r#"{}"#).unwrap();
        let mut result = walk_value_tree(&val_tree, None).unwrap();
        result.calculate_hash();
        let output_string = TypeScriptNode::to_type_string(result, false);
        assert_eq!(output_string, "type DefaultType = {\n};\n");
    }

    #[test]
    fn parses_array() {
        let val_tree = serde_json::from_str(r#"[]"#).unwrap();
        let mut result = walk_value_tree(&val_tree, None).unwrap();
        result.calculate_hash();
        let output_string = TypeScriptNode::to_type_string(result, false);
        assert_eq!(output_string, "type DefaultType = any[];\n");
    }

    #[test]
    fn parses_object_with_array() {
        let val_tree = serde_json::from_str(r#"{ "test": [] }"#).unwrap();
        let mut result = walk_value_tree(&val_tree, None).unwrap();
        result.calculate_hash();
        let output_string = TypeScriptNode::to_type_string(result, false);
        assert_eq!(output_string, "type DefaultType = {\n  test: any[];\n };\n");
    }

    #[test]
    fn parses_object_with_object() {
        let val_tree = serde_json::from_str(r#"{ "test": { "test": "test" } }"#).unwrap();
        let mut result = walk_value_tree(&val_tree, None).unwrap();
        result.calculate_hash();
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
        let mut result = walk_value_tree(&val_tree, None).unwrap();
        result.calculate_hash();
        let output_string = TypeScriptNode::to_type_string(result, false);
        assert_eq!(
            output_string,
            "type DefaultType = {\n  test: DefaultType_0[];\n };\n\ntype DefaultType_0 = { test: string; }\n"
        );
    }

    #[test]
    fn parses_object_with_array_of_arrays() {
        let val_tree = serde_json::from_str(r#"{ "test": [[], []] }"#).unwrap();
        let mut result = walk_value_tree(&val_tree, None).unwrap();
        result.calculate_hash();
        let output_string = TypeScriptNode::to_type_string(result, false);
        assert_eq!(
            output_string,
            "type DefaultType = {\n  test: any[][];\n };\n"
        );
    }

    #[test]
    fn readme_example() {
        let val_tree = serde_json::from_str(
            r#"{
                "paymentOne": {
                  "amount": 1337,
                  "status": "paid"
                },
                "paymentTwo": {
                  "amount": 1337,
                  "status": "paid"
                }
              }              
          "#,
        )
        .unwrap();
        let mut result = walk_value_tree(&val_tree, None).unwrap();
        result.calculate_hash();
        let output_string = TypeScriptNode::to_type_string(result, false);
        assert_eq!(
            output_string,
            "type DefaultType = {\n  paymentOne: DefaultType_0;\n   paymentTwo: DefaultType_0;\n };\n\ntype DefaultType_0 = {\n     amount: number;\n     status: string;\n    }\n".to_string()
        );
    }
}
