use clap::Parser;

#[derive(Parser, Debug)]
struct Args {
    /// Name of the person to greet
    #[clap(short = 'i', long = "input", value_parser)]
    input_file: String,

    #[clap(short = 'o', long = "output", value_parser)]
    output_file: u8,
}


fn main() {
    let args = Args::parse();
    println!("{:?}", args);
}
