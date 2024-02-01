// https://redis.io/docs/reference/protocol-spec

const CRLF: &str = "\r\n";

trait RespValue {
    const FIRST_BYTE: char;
    fn to_output(&self) -> String;
}

pub enum Resp<'a> {
    SimpleString(SimpleString<'a>),
    BulkString(BulkString<'a>),
    Integer(Integer),
}
impl<'a> Resp<'a> {
    // fn first_byte(&self) -> char {
    //     match self {
    //         Resp::SimpleString(_) => SimpleString::FIRST_BYTE,
    //         Resp::BulkString(_) => BulkString::FIRST_BYTE,
    //         Resp::Integer(_) => Integer::FIRST_BYTE,
    //     }
    // }
    fn to_output(&self) -> String {
        match self {
            Resp::SimpleString(inner) => inner.to_output(),
            Resp::BulkString(inner) => inner.to_output(),
            Resp::Integer(inner) => inner.to_output(),
        }
    }
}

pub struct SimpleString<'a>(pub &'a str);
impl<'a> RespValue for SimpleString<'a> {
    const FIRST_BYTE: char = '+';
    fn to_output(&self) -> String {
        format!("{}{}{CRLF}", SimpleString::FIRST_BYTE, self.0)
    }
}

pub struct BulkString<'a>(pub &'a str);
impl<'a> RespValue for BulkString<'a> {
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
impl<'a> BulkString<'a> {
    pub fn from_bytes_iter<T: Iterator<Item = &'a str>>(it: &mut T) -> Result<Self, &'static str> {
        let bs_len = it.next().ok_or("require 2 args for bulk string")?;
        let bs_str = it.next().ok_or("require 2 args for bulk string")?;
        if bs_len.chars().nth(0).ok_or("cannot access first byte")? != Self::FIRST_BYTE {
            return Err("first byte must be $");
        }
        let bs_len = bs_len[1..]
            .parse::<usize>()
            .map_err(|_| "bulk string length is unparsable")?;
        if bs_len != bs_str.len() {
            return Err("bulk string length is not the same as the bulk string");
        }
        Ok(Self(bs_str))
    }
}

struct Integer(pub i64);
impl RespValue for Integer {
    const FIRST_BYTE: char = ':';
    fn to_output(&self) -> String {
        format!("{}{}{CRLF}", BulkString::FIRST_BYTE, self.0)
    }
}
