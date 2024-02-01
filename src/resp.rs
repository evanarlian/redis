// https://redis.io/docs/reference/protocol-spec

trait RespIO {
    fn to_resp(buf: Vec<&str>) -> Result<Resp, &str>;
    fn to_output(&self) -> String;
}
enum Resp<'a> {
    SimpleString(SimpleString<'a>),
    SimpleError(SimpleError<'a>),
    Integer(Integer),
    BulkString(BulkString<'a>),
    Array(Array<'a>),
    Null(Null),
    Boolean(Boolean),
    Double(Double),
    BulkError(BulkError<'a>),
}
impl<'a> Resp<'a> {
    fn first_byte(&self) -> char {
        match self {
            Resp::SimpleString(_) => '+',
            Resp::SimpleError(_) => '-',
            Resp::Integer(_) => ':',
            Resp::BulkString(_) => '$',
            Resp::Array(_) => '*',
            Resp::Null(_) => '_',
            Resp::Boolean(_) => '#',
            Resp::Double(_) => ',',
            Resp::BulkError(_) => '!',
        }
    }
}
impl<'a> RespIO for Resp<'a> {
    fn to_resp(buf: Vec<&str>) -> Result<Resp, &str> {
        let first_byte = buf
            .first()
            .ok_or("argument length is zero")?
            .chars()
            .nth(0)
            .ok_or("first arg length is zero")?;
        match first_byte {
            '+' => SimpleString::to_resp(buf),
            '-' => SimpleError::to_resp(buf),
            ':' => Integer::to_resp(buf),
            '$' => BulkString::to_resp(buf),
            '*' => Array::to_resp(buf),
            '_' => Null::to_resp(buf),
            '#' => Boolean::to_resp(buf),
            ',' => Double::to_resp(buf),
            '!' => BulkError::to_resp(buf),
            _ => Err("first byte is not recognized"),
        }
    }
    fn to_output(&self) -> String {
        match self {
            Resp::SimpleString(_) => todo!(),
            Resp::SimpleError(_) => todo!(),
            Resp::Integer(_) => todo!(),
            Resp::BulkString(_) => todo!(),
            Resp::Array(_) => todo!(),
            Resp::Null(_) => todo!(),
            Resp::Boolean(_) => todo!(),
            Resp::Double(_) => todo!(),
            Resp::BulkError(_) => todo!(),
        }
    }
}

struct SimpleString<'a>(&'a str);
impl<'a> RespIO for SimpleString<'a> {
    fn to_resp(buf: Vec<&str>) -> Result<Resp, &str> {
        // must not contain \r and or \n
        todo!()
    }
    fn to_output(&self) -> String {
        todo!()
    }
}

struct SimpleError<'a>(&'a str);
impl<'a> RespIO for SimpleError<'a> {
    fn to_resp(buf: Vec<&str>) -> Result<Resp, &str> {
        todo!()
    }
    fn to_output(&self) -> String {
        todo!()
    }
}

struct Integer(i64);
impl RespIO for Integer {
    fn to_resp(buf: Vec<&str>) -> Result<Resp, &str> {
        todo!()
    }
    fn to_output(&self) -> String {
        todo!()
    }
}


// TODO turns out implementation of Bulk String is just String
pub struct BulkString<'a>(Vec<&'a str>);
impl<'a> RespIO for BulkString<'a> {
    fn to_resp(buf: Vec<&str>) -> Result<Resp, &str> {
        todo!()
    }
    fn to_output(&self) -> String {
        todo!()
    }
}

impl<'a> BulkString<'a> {
    // TODO NUKE
    pub fn from_array(array: &'a Array) -> Result<BulkString<'a>, &'static str> {
        let content = array.content();
        let mut bulk_strings = vec![];
        for i in (0..content.len()).step_by(2) {
            // want to convert str $4 to integer 4
            let str_len = content[i][1..].parse::<usize>().unwrap();
            let bs = content[i + 1];
            if str_len != bs.len() {
                return Err("bulk string length does not match");
            }
            bulk_strings.push(bs);
        }
        Ok(BulkString(bulk_strings))
    }
    pub fn content(&self) -> &Vec<&'a str> {
        &self.0
    }
}

pub struct Array<'a>(Vec<&'a str>);
impl<'a> RespIO for Array<'a> {
    fn to_resp(buf: Vec<&str>) -> Result<Resp, &str> {
        todo!()
    }
    fn to_output(&self) -> String {
        todo!()
    }
}

impl<'a> Array<'a> {
    // TODO move to standalone function
    pub fn from_client_bytes(buffer: &[u8]) -> Result<Array, &'static str> {
        // wild assumption: all inputs must be valid string
        let splitted = std::str::from_utf8(buffer)
            .map_err(|_| "not a valid utf-8 string")?
            .split("\r\n")
            .collect::<Vec<_>>();
        let first_byte = splitted
            .first()
            .ok_or("args length is zero")?
            .chars()
            .nth(0)
            .ok_or("first byte cannot be found")?;
        if first_byte != '*' {
            return Err("input is not redis Array type");
        }
        // now we have: *2 $4 ECHO $3 hey ""
        let array_len = splitted[0][1..].parse::<usize>().unwrap();
        if splitted.len() != array_len * 2 + 2 {
            return Err("array length is does not match content");
        }
        // trim the last unused "" because of splitting
        Ok(Array(splitted[..splitted.len() - 1].to_vec()))
    }
    fn content(&self) -> Vec<&'a str> {
        // skip the first item because it is redis Array's length metadata
        self.0[1..].to_vec()
    }
}

struct Null;
impl RespIO for Null {
    fn to_resp(buf: Vec<&str>) -> Result<Resp, &str> {
        todo!()
    }
    fn to_output(&self) -> String {
        todo!()
    }
}

struct Boolean(bool);
impl RespIO for Boolean {
    fn to_resp(buf: Vec<&str>) -> Result<Resp, &str> {
        todo!()
    }
    fn to_output(&self) -> String {
        todo!()
    }
}

struct Double(f64);
impl RespIO for Double {
    fn to_resp(buf: Vec<&str>) -> Result<Resp, &str> {
        todo!()
    }
    fn to_output(&self) -> String {
        todo!()
    }
}

struct BulkError<'a>(&'a str);
impl<'a> RespIO for BulkError<'a> {
    fn to_resp(buf: Vec<&str>) -> Result<Resp, &str> {
        todo!()
    }
    fn to_output(&self) -> String {
        todo!()
    }
}

// NOTE: the one thing that i am super confused with: client will always send array of bul string right? then when is other type used? is this the job of the client based on data types passed in? 
// For example SET num 1 in redis-cli is always string. But in python client MAYBE the set is sensitive to data type, e.g. isinstance(input, int), etc