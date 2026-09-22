use std::{
    collections::HashMap,
    io::{Read, Write},
    sync::Arc,
};

use bytes::{BufMut, Bytes, BytesMut};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::{Mutex, MutexGuard},
    time::{Duration, sleep},
};

use crate::{
    command::extract_cmd,
    resp::{Decoder, RespParser},
};

pub mod command;
pub mod resp;

#[derive(Debug, Clone)]
pub struct RawState {
    db: HashMap<String, String>,
}

pub struct State {
    inner: Arc<Mutex<RawState>>,
}

impl State {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RawState { db: HashMap::new() })),
        }
    }

    pub fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }

    pub async fn get(&self, key: &str) -> Option<String> {
        self.inner.lock().await.db.get(key).cloned()
    }

    pub async fn set(&mut self, key: String, value: String, ttl: Option<u64>) {
        self.inner.lock().await.db.insert(key.clone(), value);

        if let Some(ttl) = ttl {
            let inner = Arc::clone(&self.inner);
            tokio::spawn(async move {
                sleep(Duration::from_millis(ttl)).await;
                inner.lock().await.db.remove(&key);
            });
        }
    }
}

/// Main client connection handling routine
pub async fn handle_connection(mut stream: TcpStream, data: &mut State) {
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
            use resp::buf::RedisValueRef;
            let (command, args) = extract_cmd(v).unwrap();
            println!("Command: {}, Args: {:?}", command, args);
            let mut args = args.iter();
            match command.to_lowercase().as_str() {
                "ping" => RedisValueRef::String(bytes::Bytes::copy_from_slice(&b"PONG"[..])),
                "echo" => args.next().unwrap().clone(),
                "get" => {
                    // Extract key and value from args
                    let key = match args.next().unwrap() {
                        RedisValueRef::String(value) => String::from_utf8_lossy(value).into_owned(),
                        _ => String::new(),
                    };

                    if let Some(value) = data.get(&key).await {
                        let response = format!("{}", value);
                        RedisValueRef::String(bytes::Bytes::from(response))
                    } else {
                        RedisValueRef::String(bytes::Bytes::copy_from_slice(&b"$-1"[..]))
                    }
                }
                "set" => {
                    // Extract key and value from args
                    let key = match args.next().unwrap() {
                        RedisValueRef::String(value) => String::from_utf8_lossy(value).into_owned(),
                        _ => String::new(),
                    };

                    let value = match args.next().unwrap() {
                        RedisValueRef::String(value) => String::from_utf8_lossy(value).into_owned(),
                        _ => String::new(),
                    };

                    if let Some(exp) = args.next() {
                        if let RedisValueRef::String(_value) = exp {
                            let exp_str = String::from_utf8_lossy(_value).into_owned();
                            if exp_str.to_lowercase() == "px" {
                                if let Some(ttl_value) = args.next() {
                                    if let RedisValueRef::String(ttl_bytes) = ttl_value {
                                        if let Ok(ttl) =
                                            String::from_utf8_lossy(ttl_bytes).parse::<u64>()
                                        {
                                            data.set(key, value, Some(ttl)).await;
                                        }
                                    }
                                }
                            } else {
                                data.set(key, value, None).await;
                            }
                        }
                    } else {
                        data.set(key, value, None).await;
                    }

                    RedisValueRef::String(bytes::Bytes::copy_from_slice(&b"+OK"[..]))
                }
                c => {
                    RedisValueRef::Error(bytes::Bytes::from(format!("ERR unknown command '{}'", c)))
                }
            }
        } else {
            println!("Response couldn't be parsed due to internal error...");
            break;
        };

        let response = response.serialize();

        stream.write(response.as_bytes()).await.unwrap();
    }
}
