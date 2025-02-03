use std::error::Error;

use tokio;

pub mod comm;
pub mod frontend;
pub mod runtime;

// testing

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    comm::process_remote().await
}
