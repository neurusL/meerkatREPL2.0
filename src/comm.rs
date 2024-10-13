use std::error::Error;

use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream},
};

pub async fn process_remote() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:1145").await?;
    let (mut socket, _) = listener.accept().await?;
    loop {
        let mut data = vec![0; 1024];
        let foo = socket.read(&mut data).await?;
        println!("{}", String::from_utf8(data).unwrap());
    }
}
