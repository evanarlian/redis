use super::dtypes::BulkString;

type ClientArray = Vec<BulkString>;

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
        .next()
        .ok_or("array first byte cannot be found")?;
    if first_byte != '*' {
        return Err("input is not valid redis Array type");
    }
    // now we have: *2 $4 ECHO $3 hey ""
    let array_len = splitted[0][1..].parse::<usize>().unwrap();
    if splitted.len() != array_len * 2 + 2 {
        return Err("array length is does not match content");
    }
    // trim the last unused "" because of splitting
    Ok(splitted[1..splitted.len() - 1].to_vec())
}

pub fn parse_client_bytes(buffer: &[u8]) -> Result<ClientArray, &'static str> {
    let mut splitted = to_valid_slices(buffer)?.into_iter();
    let mut bulk_strings = vec![];
    while let Ok(bs) = BulkString::from_bytes_iter(&mut splitted) {
        bulk_strings.push(bs);
    }
    Ok(bulk_strings)
}
