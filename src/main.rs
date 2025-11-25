use clap::Parser;
use log::LevelFilter;
use std::error::Error;

use tokio;

pub mod ast;
pub mod new_runtime;
pub mod parser;
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

    let prog = parser::parser::parse(file_name.clone()) // using iterator instead of string for updated parse
        .map_err(|e| format!("Parse error: {e}"))?;

    let _ = static_analysis::typecheck::typecheck_prog(&prog);
    let _ = new_runtime::run(&prog).await;

    // runtime.repl
    Ok(())
}
