use anyhow::{Context, Result};
use clap::Parser;
use tracing::{event, span, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser, Debug)]
struct Args {
    /// Name of the person to greet
    #[clap(short = 'i', long = "input", value_parser)]
    input_file: String,

    #[clap(short = 'o', long = "output", value_parser)]
    output_file: String,
}

fn main() -> Result<()> {
    let subscrber = FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscrber)
    .expect("setting tracing default failed");

    let span = span!(Level::INFO, "parsing");

    let _enter = span.enter();

    let args = Args::parse();
    let input_file_content = std::fs::read_to_string(&args.input_file)
        .with_context(|| format!("could not read file `{}`", &args.input_file))?;

    event!(Level::INFO, input_file_content = input_file_content, "input file content");
    
    Ok(())
}
