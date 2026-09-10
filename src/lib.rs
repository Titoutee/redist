use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

pub mod resp;

/// Main client connection handling routine
pub async fn handle_connection(mut stream: TcpStream) {
    loop {
        let mut buf = [0; 512];
        let read_count = stream.read(&mut buf).await.unwrap();
        if read_count == 0 {
            break;
        }

        stream.write(b"+PONG\r\n").await.unwrap();
    }
}
