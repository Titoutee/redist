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
    let mut parser = RespParser::default();
    loop {
        let mut buf = [0; 512];
        let read_count = stream.read(&mut buf).await.unwrap();
        if read_count == 0 {
            break;
        }

        let mut buf_bytes = BytesMut::new();
        buf_bytes.put(&buf[..]);

        let value = parser.decode(&mut buf_bytes).unwrap();

        let response = if let Some(v) = value {
            let (command, args) = extract_cmd(v).unwrap();
            match command.as_str() {
                "ping" => {
                    resp::buf::RedisValueRef::String(bytes::Bytes::copy_from_slice(&b"PONG"[..]))
                }
                "echo" => args.first().unwrap().clone(),
                c => panic!("Cannot handle command {}", c),
            }
        } else {
            break;
        };

        let response = response.serialize();

        stream.write(response.as_bytes()).await.unwrap();
    }
}
