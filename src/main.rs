use std::sync::Arc;

use redist::{State, handle_connection};
use tokio;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();
    let mut data = State::new();

    loop {
        let stream = listener.accept().await;

        match stream {
            Ok((stream, _)) => {
                let mut data = data.clone();
                // No need for socket info
                println!("Accepted new connection");

                tokio::spawn(async move { handle_connection(stream, &mut data).await });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
