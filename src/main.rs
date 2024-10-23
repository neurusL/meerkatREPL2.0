use std::error::Error;

use tokio;

pub mod backend;
pub mod comm;
pub mod frontend;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    comm::process_remote().await
}
