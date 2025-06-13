use clap::Parser;
use log::LevelFilter;
use std::error::Error;
use std::{env, fs};

use parser::meerkat;

use tokio;

pub mod ast;
pub mod parser;
pub mod runtime;
pub mod static_analysis;

/// example usage
/// cargo run -- -f test0.meerkat -v
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(short = 'f', long = "file", default_value = "test0.meerkat")]
    input_file: String,

    /// verbose for debug info printing
    /// all such printing go to info!("...")
    #[arg(short = 'v', long = "verbose", default_value_t = false)]
    verbose: bool,
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let log_level = if args.verbose {
        LevelFilter::Info
    } else {
        log::LevelFilter::Warn
    };
    env_logger::Builder::from_default_env()
        .filter_level(log_level)
        .init();

    let file_name = args.input_file; // the second argument be test.meerkat
    let file_content = fs::read_to_string(file_name).expect("Couldn't read file");

    let meerkat_parser = meerkat::ProgParser::new();
    let prog = match meerkat_parser.parse(&file_content) {
        Ok(ast) => ast,
        Err(e) => panic!("Parse Error: {:?}", e),
    };

    let _ = static_analysis::typecheck::typecheck_prog(&prog);
    let _ = runtime::run(&prog).await;

    // runtime.repl
    Ok(())
}
