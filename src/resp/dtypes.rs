// https://redis.io/docs/reference/protocol-spec

const CRLF: &str = "\r\n";

pub trait RespValue {
    const FIRST_BYTE: char;
    fn to_output(&self) -> String;
}

pub enum Resp {
    SimpleString(SimpleString),
    SimpleError(SimpleError),
    BulkString(BulkString),
    Integer(Integer),
}
impl Resp {
    pub fn to_output(&self) -> String {
        match self {
            Resp::SimpleString(inner) => inner.to_output(),
            Resp::SimpleError(inner) => inner.to_output(),
            Resp::BulkString(inner) => inner.to_output(),
            Resp::Integer(inner) => inner.to_output(),
        }
    }
}

pub struct SimpleString(pub String);
impl RespValue for SimpleString {
    const FIRST_BYTE: char = '+';
    fn to_output(&self) -> String {
        format!("{}{}{CRLF}", SimpleString::FIRST_BYTE, self.0)
    }
}

pub struct SimpleError(pub String);
impl RespValue for SimpleError {
    const FIRST_BYTE: char = '-';
    fn to_output(&self) -> String {
        format!("{}{}{CRLF}", SimpleError::FIRST_BYTE, self.0)
    }
}

pub struct BulkString(pub String);
impl RespValue for BulkString {
    const FIRST_BYTE: char = '$';
    fn to_output(&self) -> String {
        format!(
            "{}{}{CRLF}{}{CRLF}",
            BulkString::FIRST_BYTE,
            self.0.len(),
            self.0
        )
    }
}
impl BulkString {
    pub fn from_bytes_iter<'a, T>(it: &mut T) -> Result<Self, SimpleError>
    where
        T: Iterator<Item = &'a str>,
    {
        let bs_len = it
            .next()
            .ok_or(SimpleError("ERR require 2 args for BulkString".into()))?;
        let bs_str = it
            .next()
            .ok_or(SimpleError("ERR require 2 args for BulkString".into()))?;
        if bs_len.chars().nth(0).ok_or(SimpleError(
            "ERR cannot access BulkString first byte".into(),
        ))? != Self::FIRST_BYTE
        {
            return Err(SimpleError(format!(
                "ERR first byte for BulkString must be {}",
                Self::FIRST_BYTE
            )));
        }
        let bs_len = bs_len[1..]
            .parse::<usize>()
            .map_err(|_| SimpleError("ERR BulkString length is unparsable".into()))?;
        if bs_len != bs_str.len() {
            return Err(SimpleError(
                "ERR BulkString length is not the same as the BulkString".into(),
            ));
        }
        Ok(Self(bs_str.to_owned()))
    }
}

struct Integer(pub i64);
impl RespValue for Integer {
    const FIRST_BYTE: char = ':';
    fn to_output(&self) -> String {
        format!("{}{}{CRLF}", BulkString::FIRST_BYTE, self.0)
    }
}
