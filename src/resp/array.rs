use super::dtypes::BulkString;
use super::dtypes::Resp;

struct ClientArray<'a>(Vec<BulkString<'a>>);

impl<'a> ClientArray<'a> {
    fn to_valid_slices(buffer: &[u8]) -> Result<Vec<&str>, &'static str> {
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
            return Err("input is not valid redis Array type");
        }
        // now we have: *2 $4 ECHO $3 hey ""
        let array_len = splitted[0][1..].parse::<usize>().unwrap();
        if splitted.len() != array_len * 2 + 2 {
            return Err("array length is does not match content");
        }
        // trim the last unused "" because of splitting
        Ok(splitted[0..splitted.len() - 1].to_vec())
    }
    pub fn parse_client_bytes(buffer: &[u8]) -> Result<ClientArray, &'static str> {
        let splitted = ClientArray::to_valid_slices(buffer)?;
        let mut bulk_strings = vec![];
        for i in (0..splitted.len()).step_by(2) {
            if splitted[i]
                .chars()
                .nth(0)
                .ok_or("cannot get first byte bulk string")?
                != '$'
            {
                return Err("bulk string byte is not $");
            }
            let bs_len = splitted[i][1..]
                .parse::<usize>()
                .map_err(|_| "failed to parse bulk string length")?;
            if bs_len != splitted[i + 1].len() {
                return Err("bulk string length does not match content");
            }
            bulk_strings.push(BulkString(splitted[i + 1]));
        }
        Ok(ClientArray(bulk_strings))
    }
}
