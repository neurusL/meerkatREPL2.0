use std::error::Error;

use tokio;

pub mod ast;
pub mod parse;
pub mod comm;
pub mod frontend;
pub mod runtime;

// testing

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    // runtime.repl
    Ok(())
}
