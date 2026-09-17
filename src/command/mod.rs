use crate::resp::buf::RedisValueRef;

/// Extracts and separates command and arguments from command array
pub fn extract_cmd(value: RedisValueRef) -> anyhow::Result<(String, Vec<RedisValueRef>)> {
    match value {
        RedisValueRef::Array(a) => Ok((
            unpack_bulk_str(a.first().unwrap().clone())?,
            a.into_iter().skip(1).collect(),
        )),
        _ => Err(anyhow::anyhow!(
            "Unexpected command format (possibly missing command array?)"
        )),
    }
}

fn unpack_bulk_str(value: RedisValueRef) -> anyhow::Result<String> {
    match value {
        RedisValueRef::String(s) => Ok(String::from_utf8(s.to_vec())?),
        _ => Err(anyhow::anyhow!("Expected command to be a bulk string")),
    }
}
