use anyhow::{Context, Result};
use clap::Parser;
use serde_json::Value;
use std::{collections::HashMap, iter::Map};
use tracing::{event, span, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser, Debug)]
struct Args {
    #[clap(short = 'i', long = "input", value_parser)]
    input_file: String,

    #[clap(short = 'o', long = "output", value_parser)]
    output_file: String,
}

#[derive(Debug)]
struct TypeScriptTypeNode {
    type_name: String,
    optional: bool,
    nullable: bool,
    sub_items: Option<HashMap<String, TypeScriptTypeNode>>,
}

fn main() -> Result<()> {
    let subscrber = FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscrber).expect("setting tracing default failed");

    let span = span!(Level::INFO, "parsing");

    let _enter = span.enter();

    let args = Args::parse();
    let input_file_content = std::fs::read_to_string(&args.input_file)
        .with_context(|| format!("could not read file `{}`", &args.input_file))?;

    event!(
        Level::INFO,
        input_file_content = input_file_content,
        "input file content"
    );

    let mut type_tree = HashMap::<String, TypeScriptTypeNode>::new();

    let v: Value = serde_json::from_str(input_file_content.as_str())
        .with_context(|| format!("could not parse json"))?;

    walk_value_tree(&v, &mut type_tree);

    print_type_tree(&type_tree, 0);
    Ok(())
}

#[tracing::instrument]
fn walk_value_tree(v: &Value, type_tree: &mut HashMap<String, TypeScriptTypeNode>) {
    match v {
        Value::Object(o) => {
            for (k, v) in o {
                if is_value_type(v) {
                    type_tree.insert(
                        k.to_string(),
                        TypeScriptTypeNode {
                            type_name: classify_value_type(v),
                            optional: false,
                            nullable: false,
                            sub_items: None,
                        },
                    );
                    event!(
                        Level::INFO,
                        key = k,
                        value = classify_value_type(v),
                        "found value"
                    );
                } else {
                    let mut sub_items = HashMap::<String, TypeScriptTypeNode>::new();
                    walk_value_tree(v, &mut sub_items);
                    type_tree.insert(
                        k.to_string(),
                        TypeScriptTypeNode {
                            type_name: "object".to_string(),
                            optional: false,
                            nullable: false,
                            sub_items: Some(sub_items),
                        },
                    );
                }
            }
        }
        Value::Array(a) => {
            let mut sub_items = HashMap::<String, TypeScriptTypeNode>::new();
            for v in a {
                if is_value_type(v) {
                    sub_items.insert(
                        "item".to_string(),
                        TypeScriptTypeNode {
                            type_name: classify_value_type(v),
                            optional: false,
                            nullable: false,
                            sub_items: None,
                        },
                    );
                    event!(Level::INFO, value = classify_value_type(v), "found value");
                } else {
                    walk_value_tree(v, &mut sub_items);
                }
            }
        }
        _ => {
            TypeScriptTypeNode {
                type_name: classify_value_type(v),
                optional: false,
                nullable: false,
                sub_items: None,
            };
            event!(Level::INFO, "{}", classify_value_type(v));
        }
    }
}

#[tracing::instrument]
fn classify_value_type(v: &Value) -> String {
    match v {
        Value::Bool(_) => "boolean".to_string(),
        Value::String(_) => "string".to_string(),
        Value::Number(_) => "number".to_string(),
        Value::Null => "never".to_string(),
        _ => "unknown".to_string(),
    }
}

fn is_value_type(v: &Value) -> bool {
    match v {
        Value::Bool(_) => true,
        Value::String(_) => true,
        Value::Number(_) => true,
        Value::Null => true,
        _ => false,
    }
}

fn print_type_tree(type_tree: &HashMap<String, TypeScriptTypeNode>, indent: usize) {
    let mut indent_str = String::new();
    for _ in 0..indent {
        indent_str.push_str("  ");
    }
    for (k, v) in type_tree {
        println!("{}{}:{}", indent_str, k.to_string(), v.type_name);
        match &v.sub_items {
            Some(sub_items) => {
                println!("{{");
                print_type_tree(&sub_items, indent + 1);
                println!("}}");
            }
            None => (),
        }
    }
}
