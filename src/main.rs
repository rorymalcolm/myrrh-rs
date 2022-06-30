use anyhow::{Context, Result};
use clap::Parser;
use serde_json::Value;
use tracing::{event, span, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser, Debug)]
struct Args {
    #[clap(short = 'i', long = "input", value_parser)]
    input_file: String,

    #[clap(short = 'o', long = "output", value_parser)]
    output_file: String,
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

    let v: Value = serde_json::from_str(input_file_content.as_str())
        .with_context(|| format!("could not parse json"))?;

    walk_value_tree(&v);

    Ok(())
}

#[tracing::instrument]
fn walk_value_tree(v: &Value) {
    match v {
        Value::Object(o) => {
            for (k, v) in o {
                event!(Level::INFO, "{}: ", k);
                walk_value_tree(v);
            }
        }
        Value::Array(a) => {
            for v in a {
                walk_value_tree(v);
            }
        }
        _ => {
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
