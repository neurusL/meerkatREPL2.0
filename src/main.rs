use std::error::Error;
use std::{env, fs};

use parser::meerkat;
use tokio;

pub mod ast;
pub mod parser;
pub mod typecheck;
pub mod runtime;



#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    let file_name = &args[1]; // the second argument be test.meerkat
    let file_content = fs::read_to_string(file_name).expect("Couldn't read file");

    let meerkat_parser = meerkat::ProgParser::new();
    let prog = match meerkat_parser.parse(&file_content) {
        Ok(ast) => ast,
        Err(e) => panic!("Parse Error: {:?}", e)
    };

    let _ = typecheck::typecheck_prog(&prog);
    let _ = runtime::evaluator::eval_prog(&prog);
    
    // runtime.repl
    Ok(())
}
