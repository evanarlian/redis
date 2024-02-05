const CRLF: &str = "\r\n";

pub trait RespValue {
    const FIRST_BYTE: char;
    fn to_output(&self) -> String;
}

// https://redis.io/docs/reference/protocol-spec
pub enum Resp {
    SimpleString(SimpleString),
    SimpleError(SimpleError),
    BulkString(BulkString),
    Array(Array),
    Null(Null),
    Integer(Integer),
}
impl Resp {
    pub fn to_output(&self) -> String {
        match self {
            Resp::SimpleString(inner) => inner.to_output(),
            Resp::SimpleError(inner) => inner.to_output(),
            Resp::BulkString(inner) => inner.to_output(),
            Resp::Array(inner) => inner.to_output(),
            Resp::Null(inner) => inner.to_output(),
            Resp::Integer(inner) => inner.to_output(),
        }
    }
}

pub struct SimpleString(pub String);
impl RespValue for SimpleString {
    const FIRST_BYTE: char = '+';
    fn to_output(&self) -> String {
        format!("{}{}{CRLF}", Self::FIRST_BYTE, self.0)
    }
}

pub struct SimpleError(pub String);
impl RespValue for SimpleError {
    const FIRST_BYTE: char = '-';
    fn to_output(&self) -> String {
        format!("{}{}{CRLF}", Self::FIRST_BYTE, self.0)
    }
}

pub struct BulkString(pub String);
impl RespValue for BulkString {
    const FIRST_BYTE: char = '$';
    fn to_output(&self) -> String {
        format!("{}{}{CRLF}{}{CRLF}", Self::FIRST_BYTE, self.0.len(), self.0)
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

pub struct Array(pub Vec<Resp>);
impl RespValue for Array {
    const FIRST_BYTE: char = '*';
    fn to_output(&self) -> String {
        let mut output = format!("{}{}{CRLF}", Self::FIRST_BYTE, self.0.len());
        for resp in &self.0 {
            output.push_str(&resp.to_output());
        }
        output
    }
}
impl Array {
    fn to_valid_slices(buffer: &[u8]) -> Result<Vec<&str>, SimpleError> {
        // wild assumption: all inputs must be valid string
        let splitted = std::str::from_utf8(buffer)
            .map_err(|_| SimpleError("input is not valid utf-8".into()))?
            .split("\r\n")
            .collect::<Vec<_>>();
        let first_byte = splitted
            .first()
            .ok_or(SimpleError("input args length is 0".into()))?
            .chars()
            .next()
            .ok_or(SimpleError("ERR redis Array first byte missing".into()))?;
        if first_byte != '*' {
            return Err(SimpleError(
                "ERR input is not valid redis Array type".into(),
            ));
        }
        // now we have: *2 $4 ECHO $3 hey ""
        let array_len = splitted[0][1..].parse::<usize>().unwrap();
        if splitted.len() != array_len * 2 + 2 {
            return Err(SimpleError("array length is does not match content".into()));
        }
        // trim the last unused "" because of splitting
        Ok(splitted[1..splitted.len() - 1].to_vec())
    }
    pub fn parse_client_bytes(buffer: &[u8]) -> Result<Vec<String>, SimpleError> {
        let mut splitted = Array::to_valid_slices(buffer)?.into_iter();
        let mut bulk_strings = vec![];
        while let Ok(bs) = BulkString::from_bytes_iter(&mut splitted) {
            bulk_strings.push(bs.0);
        }
        Ok(bulk_strings)
    }
}

pub struct Null;
impl RespValue for Null {
    const FIRST_BYTE: char = '_';
    fn to_output(&self) -> String {
        // HACK: this is just to satisfy codecrafters, I think they are using old RESP 2 protocol
        "$-1\r\n".into() // (nil bulk string)

        // format!("{}{CRLF}", Self::FIRST_BYTE)
    }
}

struct Integer(pub i64);
impl RespValue for Integer {
    const FIRST_BYTE: char = ':';
    fn to_output(&self) -> String {
        format!("{}{}{CRLF}", Self::FIRST_BYTE, self.0)
    }
}
