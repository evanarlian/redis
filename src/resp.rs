enum Resp<'a> {
    Array(Array<'a>),
    BulkString(BulkString<'a>),
}

pub struct Array<'a>(Vec<&'a str>);
impl<'a> Array<'a> {
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

pub struct BulkString<'a>(Vec<&'a str>);
impl<'a> BulkString<'a> {
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
