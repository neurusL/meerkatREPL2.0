use std::error::Error;
use std::{env, fs};

use kameo::spawn;
use parser::meerkat;
use runtime::manager::Manager;
use tokio;

pub mod ast;
pub mod parser;
pub mod runtime;
pub mod static_analysis;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    let file_name = &args[1]; // the second argument be test.meerkat

    let prog = parser::parser::parse(file_name.clone())  // using iterator instead of string for updated parse
        .map_err(|e| format!("Parse error: {e}"))?;

    let _ = static_analysis::typecheck::typecheck_prog(&prog);
    let _ = static_analysis::var_analysis::calc_dep_prog(&prog);
    let _ = runtime::evaluator::eval_prog(&prog);

    let mgr = Manager::new("main_service".to_string());
    let mgr_actor_ref = spawn(mgr);

    // runtime.repl
    Ok(())
}
