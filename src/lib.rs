use std::io::Write;

use bytes::{BufMut, BytesMut};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::{
    command::extract_cmd,
    resp::{Decoder, RespParser},
};

pub mod command;
pub mod resp;

/// Main client connection handling routine
pub async fn handle_connection(mut stream: TcpStream) {
    let stdout = std::io::stdout();

    let mut parser = RespParser::default();
    loop {
        let mut buf = [0; 512];
        let read_count = stream.read(&mut buf).await.unwrap();
        if read_count == 0 {
            let _ = writeln!(&mut stdout.lock(), "Didn't read any byte...");
            break;
        }

        let mut buf_bytes = BytesMut::new();
        buf_bytes.put(&buf[..read_count]);

        println!("Received: {:?}", String::from_utf8_lossy(&buf_bytes));

        let value = parser.decode(&mut buf_bytes).unwrap();

        let response = if let Some(v) = value {
            let (command, args) = extract_cmd(v).unwrap();
            match command.to_lowercase().as_str() {
                "ping" => {
                    resp::buf::RedisValueRef::String(bytes::Bytes::copy_from_slice(&b"PONG"[..]))
                }
                "echo" => args.first().unwrap().clone(),
                c => resp::buf::RedisValueRef::Error(bytes::Bytes::from(format!(
                    "ERR unknown command '{}'",
                    c
                ))),
            }
        } else {
            println!("Response couldn't be parsed due to internal error...");
            break;
        };

        let response = response.serialize();

        stream.write(response.as_bytes()).await.unwrap();
    }
}
