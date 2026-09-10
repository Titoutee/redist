/// Fundamental struct for viewing byte slices
///
/// Used for zero-copy redis values.
pub struct BufSplit(pub usize, pub usize);

impl BufSplit {
    #[inline]
    pub fn as_slice<'a>(&self, buf: &'a BytesMut) -> &'a [u8] {
        &buf[self.0..self.1]
    }

    #[inline]
    pub fn as_bytes(&self, buf: &Bytes) -> Bytes {
        buf.slice(self.0..self.1)
    }
}

use bytes::{Bytes, BytesMut};

use crate::resp::error::RedisResult;

/// BufSplit based equivalent to output type RedisValueRef
pub enum RedisBufSplit {
    String(BufSplit),
    Error(BufSplit),
    Int(i64),
    Array(Vec<RedisBufSplit>),
    NullArray,
    NullBulkString,
}

#[derive(PartialEq, Clone)]
pub enum RedisValueRef {
    String(Bytes),
    Error(Bytes),
    Int(i64),
    Array(Vec<RedisValueRef>),
    NullArray,
    NullBulkString,
    ErrorMsg(Vec<u8>), // This is not a RESP type. This is an redis-oxide internal error type.
}

#[allow(dead_code)]
impl RedisBufSplit {
    fn redis_value(self, buf: &Bytes) -> RedisValueRef {
        match self {
            // bfs is BufSplit(start, end), which has the as_bytes method defined above
            RedisBufSplit::String(bfs) => RedisValueRef::String(bfs.as_bytes(buf)),
            RedisBufSplit::Error(bfs) => RedisValueRef::Error(bfs.as_bytes(buf)),
            RedisBufSplit::Array(arr) => {
                RedisValueRef::Array(arr.into_iter().map(|bfs| bfs.redis_value(buf)).collect())
            }
            RedisBufSplit::NullArray => RedisValueRef::NullArray,
            RedisBufSplit::NullBulkString => RedisValueRef::NullBulkString,
            RedisBufSplit::Int(i) => RedisValueRef::Int(i),
        }
    }
}

pub trait Decoder {
    type Item;
    fn decode(&mut self, src: &BytesMut) -> Self::Item;
}

pub mod parse {

    use crate::resp::error::RESPError;

    use super::RedisResult;
    use super::{BufSplit, RedisBufSplit};
    use bytes::BytesMut;
    use memchr::memchr;

    /// Parses a single word in the request buffer
    #[inline]
    fn word(buf: &BytesMut, pos: usize) -> Option<(usize, BufSplit)> {
        // At the edge of buf, so can't find a word
        if buf.len() <= pos {
            return None;
        }

        memchr(b'\r', &buf[pos..]).and_then(|end| {
            if end + 1 < buf.len() {
                Some((pos + end, BufSplit(pos, pos + end + 2)))
            } else {
                None
            }
        })
    }

    /// Parses a simple string
    fn simple_string(buf: &BytesMut, pos: usize) -> RedisResult<usize, RedisBufSplit> {
        match word(buf, pos) {
            Some((pos, word)) => Ok(Some((pos, RedisBufSplit::String(word)))),
            None => Ok(None),
        }
    }

    /// Parses a bulk string
    fn bulk_string(buf: &BytesMut, pos: usize) -> RedisResult<usize, RedisBufSplit> {
        match int(buf, pos)? {
            // Special case: empty bulk string
            Some((pos, -1)) => Ok(Some((pos, RedisBufSplit::NullBulkString))),
            Some((pos, size)) if size >= 0 => {
                let total_size = pos + size as usize;
                // the client hasn't sent the server enough bytes
                if buf.len() < total_size + 2 {
                    Ok(None)
                } else {
                    // Enough bytes: is possible to generate the correct type
                    let b = RedisBufSplit::String(BufSplit(pos, total_size));
                    Ok(Some((total_size + 2, b)))
                }
            }
            Some((_pos, bad_size)) => Err(RESPError::BadBulkStringSize(bad_size)),
            None => Ok(None),
        }
    }

    /// Parses a simple int
    fn int(buf: &BytesMut, pos: usize) -> RedisResult<usize, i64> {
        match word(buf, pos) {
            Some((pos, word)) => {
                let s =
                    str::from_utf8(word.as_slice(buf)).map_err(|_| RESPError::IntParseFailure)?;
                let i = s.parse().map_err(|_| RESPError::IntParseFailure)?;
                Ok(Some((pos, i)))
            }
            None => Ok(None),
        }
    }

    /// Parses a resp-compliant int from the request buffer, thus returning a wrapped `RedisBufSplit`
    fn resp_int(buf: &BytesMut, pos: usize) -> RedisResult<usize, RedisBufSplit> {
        Ok(int(buf, pos)?.map(|(pos, int)| (pos, RedisBufSplit::Int(int))))
    }

    /// Parses a resp error
    fn error(buf: &BytesMut, pos: usize) -> RedisResult<usize, RedisBufSplit> {
        match word(buf, pos) {
            Some((pos, word)) => Ok(Some((pos, RedisBufSplit::Error(word)))),
            None => Ok(None),
        }
    }

    fn array(buf: &BytesMut, pos: usize) -> RedisResult<usize, RedisBufSplit> {
        match int(buf, pos)? {
            None => Ok(None),
            Some((pos, -1)) => Ok(Some((pos, RedisBufSplit::NullArray))),
            Some((pos, num_elts)) if num_elts >= 0 => {
                let mut values = Vec::with_capacity(num_elts as usize);

                let mut cursor = pos;
                for _ in 0..num_elts {
                    match parse(buf, cursor)? {
                        Some((new_pos, value)) => {
                            cursor = new_pos;
                            values.push(value);
                        }
                        None => return Ok(None),
                    }
                }
                Ok(Some((cursor, RedisBufSplit::Array(values))))
            }
            Some((_pos, bad_num_elts)) => Err(RESPError::BadArraySize(bad_num_elts)),
        }
    }

    /// Main parsing function, directing appropriate inner parsing functions, according to the different official RESP tags
    pub fn parse(buf: &BytesMut, pos: usize) -> RedisResult<usize, RedisBufSplit> {
        if buf.is_empty() {
            return Ok(None);
        }

        match buf[pos] {
            b'+' => simple_string(buf, pos + 1),
            b'-' => error(buf, pos + 1),
            b'$' => bulk_string(buf, pos + 1),
            b':' => resp_int(buf, pos + 1),
            b'*' => array(buf, pos + 1),
            _ => Err(RESPError::UnknownStartingByte),
        }
    }
}
