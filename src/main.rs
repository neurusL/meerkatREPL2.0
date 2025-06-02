use std::error::Error;
use std::{env, fs};

use kameo::spawn;
use parser::meerkat;
use runtime::manager::Manager;
use tokio;

pub mod ast;
pub mod parser;
pub mod static_analysis;
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

    let _ = static_analysis::typecheck::typecheck_prog(&prog);
    let _ = static_analysis::var_analysis::calc_dep_prog(&prog);
    let _ = runtime::evaluator::eval_prog(&prog);

    let mgr = Manager::new();
    let mgr_actor_ref = spawn(mgr);
    
    // runtime.repl
    Ok(())
}
