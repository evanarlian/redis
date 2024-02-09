// https://rdb.fnordig.de/file_format.html

use core::panic;
use std::{
    collections::HashMap,
    time::{Duration, UNIX_EPOCH},
};

use crate::db::database::RedisValue;

enum ValueType {
    String,
    List,
    Set,
    SortedSet,
    Hash,
    Zipmap,
    Ziplist,
    Intset,
    SortedSetInZiplist,
    HashmapInZiplist,
    ListInQuicklist,
}
impl ValueType {
    fn from_u8(num: u8) -> ValueType {
        match num {
            0 => ValueType::String,
            1 => ValueType::List,
            2 => ValueType::Set,
            3 => ValueType::SortedSet,
            4 => ValueType::Hash,
            9 => ValueType::Zipmap,
            10 => ValueType::Ziplist,
            11 => ValueType::Intset,
            12 => ValueType::SortedSetInZiplist,
            13 => ValueType::HashmapInZiplist,
            14 => ValueType::ListInQuicklist,
            _ => panic!("invalid value type"),
        }
    }
}

enum DecodedLength {
    Number(usize),
    ToString(usize),
}

#[derive(Default)]
pub struct RdbParseResult {
    rdb_ver: u32,
    aux_map: HashMap<String, String>, // metadata-ish
    pub kv_map: HashMap<String, RedisValue>,
}

struct ByteWrapper<'a> {
    b: &'a [u8],
    i: usize,
}
impl<'a> ByteWrapper<'a> {
    fn new(buffer: &'a [u8]) -> Self {
        Self { b: buffer, i: 0 }
    }
    fn get(&mut self, n: usize) -> &'a [u8] {
        let old_i = self.i;
        self.i += n;
        &self.b[old_i..old_i + n]
    }
    fn one(&mut self) -> u8 {
        let old_i = self.i;
        self.i += 1;
        self.b[old_i]
    }
}

pub fn parse_rdb(buf: &[u8]) -> RdbParseResult {
    let mut buf = ByteWrapper::new(buf);
    if buf.get(5) != b"REDIS" {
        panic!("redis magic fail");
    }
    let rdb_ver = std::str::from_utf8(buf.get(4))
        .unwrap()
        .parse::<u32>()
        .unwrap();
    let mut aux_map: HashMap<String, String> = HashMap::new();
    let mut kv_map: HashMap<String, RedisValue> = HashMap::new();
    loop {
        match buf.one() {
            0xFA => {
                // parse aux
                let key = string_decode(&mut buf);
                let value = string_decode(&mut buf);
                eprintln!("[rdb] aux: {} {}", key, value);
                aux_map.insert(key, value);
            }
            0xFB => {
                // parse resizedb to get the looping number for how many keys
                parse_resizedb_and_keyvals(&mut buf, &mut kv_map);
            }

            0xFE => {
                // selectdb, idk what is this
                eprintln!("[rdb] 0xFE selectdb unimplemented, this one is only 1 db");
                let _db_selection = length_decode(&mut buf);
            }
            0xFF => break,
            _ => {}
        }
    }
    RdbParseResult {
        rdb_ver,
        aux_map,
        kv_map,
    }
}

fn string_decode(buf: &mut ByteWrapper) -> String {
    match length_decode(buf) {
        DecodedLength::ToString(len) => len.to_string(),
        DecodedLength::Number(len) => std::str::from_utf8(buf.get(len)).unwrap().to_string(),
    }
}

fn length_decode(buf: &mut ByteWrapper) -> DecodedLength {
    let first = buf.one();
    match first & 0b_1100_0000 {
        0b_1100_0000 => {
            // read remaining 6 bits to know what to do
            // currently only supports string encoding
            let len = match first & 0b_0011_1111 {
                0 => {
                    // 8 bit int follow
                    buf.one() as usize
                }
                1 => {
                    // 16 bit int follow
                    let buf_u16: [u8; 2] = buf.get(2).try_into().unwrap();
                    u16::from_be_bytes(buf_u16) as usize
                }
                2 => {
                    // 32 bit int follow
                    let buf_u32: [u8; 4] = buf.get(4).try_into().unwrap();
                    u32::from_be_bytes(buf_u32) as usize
                }
                _ => {
                    panic!("rdb file is not valid, length decode fail")
                }
            };
            DecodedLength::ToString(len)
        }
        0b_1000_0000 => {
            // discard remaining 6 bits and use the next 8 bits
            DecodedLength::Number(buf.one() as usize)
        }
        0b_0100_0000 => {
            // read additional byte and combined 14 bits is the length
            let second = buf.one();
            let len = ((first & 0b_0011_1111) as usize) << 8 | (second as usize);
            DecodedLength::Number(len)
        }
        0b_0000_0000 => {
            // the next 6 bits represent the length
            DecodedLength::Number((first & 0b_0011_1111) as usize)
        }
        _ => unreachable!(),
    }
}

fn parse_kv(buf: &mut ByteWrapper) -> (String, String) {
    let value_type = ValueType::from_u8(buf.one());
    let key = string_decode(buf);
    let value = match value_type {
        ValueType::String => string_decode(buf),
        _ => unimplemented!("bro i cant"),
    };
    (key, value)
}

fn parse_resizedb_and_keyvals(buf: &mut ByteWrapper, kv_map: &mut HashMap<String, RedisValue>) {
    // a little bit weird but it works
    let all_keys_cnt = match length_decode(buf) {
        DecodedLength::Number(num) => num,
        DecodedLength::ToString(num) => num,
    };
    let mut keys_with_expiry_cnt = match length_decode(buf) {
        DecodedLength::Number(num) => num,
        DecodedLength::ToString(num) => num,
    };
    let mut keys_wo_expiry_cnt = all_keys_cnt - keys_with_expiry_cnt;
    // parse all keyvals
    for _ in 0..all_keys_cnt {
        let expiry = match buf.one() {
            0xFC => {
                // expiry in millisecs
                let temp: [u8; 8] = buf.get(8).try_into().unwrap();
                let unix_millis = u64::from_be_bytes(temp);
                let expiry = UNIX_EPOCH + Duration::from_millis(unix_millis);
                Some(expiry)
            }
            0xFD => {
                // expiry in secs
                let temp = buf.get(4).try_into().unwrap();
                let unix_secs = u32::from_be_bytes(temp);
                let expiry = UNIX_EPOCH + Duration::from_secs(unix_secs as u64);
                Some(expiry)
            }
            _ => {
                // no expiry
                None
            }
        };
        let (key, value) = parse_kv(buf);
        eprintln!("[rdb] kv: {} {}", key, value);
        kv_map.insert(
            key,
            RedisValue {
                content: value,
                expiry,
            },
        );
    }
    // validate kv
    for (_k, v) in kv_map.iter() {
        if let RedisValue {
            content: _content,
            expiry: Some(_time),
        } = v
        {
            keys_with_expiry_cnt -= 1;
        } else {
            keys_wo_expiry_cnt -= 1;
        }
    }
    assert_eq!(
        keys_with_expiry_cnt, 0,
        "resizedb size mismatch (with expiry)"
    );
    assert_eq!(keys_wo_expiry_cnt, 0, "resizedb size mismatch (wo expiry)");
}
