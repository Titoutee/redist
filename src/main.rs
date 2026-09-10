use redist::handle_connection;
use tokio;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();

    loop {
        let stream = listener.accept().await;

        match stream {
            Ok((stream, _)) => {
                // No need for socket info
                println!("Accepted new connection");

                tokio::spawn(async move { handle_connection(stream) });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
