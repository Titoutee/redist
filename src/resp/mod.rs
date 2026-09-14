pub mod buf;
pub mod error;

use buf::{RedisValueRef, parse::parse};
use bytes::BytesMut;

use crate::resp::error::RESPError;

pub trait Decoder {
    type Item;
    fn decode(&mut self, src: &mut BytesMut) -> Self::Item;
}

#[derive(Default)]
pub struct RespParser;

impl Decoder for RespParser {
    type Item = Result<Option<RedisValueRef>, RESPError>;
    fn decode(&mut self, src: &mut BytesMut) -> Self::Item {
        if src.is_empty() {
            return Ok(None);
        }

        match parse(src, 0)? {
            Some((pos, value)) => {
                let data = src.split_to(pos);
                Ok(Some(value.redis_value(&data.freeze())))
            }
            None => Ok(None),
        }
    }
}
