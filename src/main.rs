use std::error::Error;
use std::{env, fs};

use kameo::spawn;
use parser::meerkat;
use runtime::manager::Manager;
use runtime::message::Msg;
use tokio;

pub mod ast;
pub mod parser;
pub mod runtime;
pub mod static_analysis;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    let file_name = &args[1]; // the second argument be test.meerkat
    let file_content = fs::read_to_string(file_name).expect("Couldn't read file");

    let meerkat_parser = meerkat::ProgParser::new();
    let prog = match meerkat_parser.parse(&file_content) {
        Ok(ast) => ast,
        Err(e) => panic!("Parse Error: {:?}", e),
    };

    let _ = static_analysis::typecheck::typecheck_prog(&prog);
    let _ = static_analysis::var_analysis::calc_dep_prog(&prog);
    let _ = runtime::evaluator::eval_prog(&prog);

    for srv in prog.services.iter() {
        let mgr = Manager::new(srv.name.clone());
        let mgr_actor_ref = spawn(mgr);
        mgr_actor_ref.tell(Msg::CodeUpdate{ srv: srv.clone() }).await?;
    }
    
    // runtime.repl
    Ok(())
}
