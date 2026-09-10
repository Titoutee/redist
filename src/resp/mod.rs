pub mod buf;
pub mod error;

use buf::{Decoder, RedisValueRef, parse::parse};
use error::RedisResult;

#[derive(Default)]
pub struct RespParser;

impl Decoder for RespParser {
    type Item = RedisValueRef;
    fn decode(&mut self, src: &bytes::BytesMut) -> Self::Item {
        todo!()
    }
}
