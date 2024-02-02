use super::dtypes::{BulkString, SimpleError};

type ClientArray = Vec<BulkString>;

fn to_valid_slices(buffer: &[u8]) -> Result<Vec<&str>, SimpleError> {
    // wild assumption: all inputs must be valid string
    let splitted = std::str::from_utf8(buffer)
        .map_err(|_| SimpleError("ERR not a valid utf-8 string".into()))?
        .split("\r\n")
        .collect::<Vec<_>>();
    let first_byte = splitted
        .first()
        .ok_or(SimpleError("ERR args length is zero".into()))?
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

pub fn parse_client_bytes(buffer: &[u8]) -> Result<ClientArray, SimpleError> {
    let mut splitted = to_valid_slices(buffer)?.into_iter();
    let mut bulk_strings = vec![];
    while let Ok(bs) = BulkString::from_bytes_iter(&mut splitted) {
        bulk_strings.push(bs);
    }
    Ok(bulk_strings)
}

// TODO refactor intos, is it btter to be ::new()