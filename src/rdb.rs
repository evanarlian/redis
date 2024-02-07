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
    ToString(usize),
    ReadMore(usize),
}

#[derive(Default)]
pub struct RdbParseResult {
    rdb_ver: u32,
    aux_map: HashMap<String, String>, // metadata-ish
    pub redis_map: HashMap<String, RedisValue>,
}

pub fn parse_rdb(buf: Vec<u8>) -> RdbParseResult {
    if &buf[..5] != b"REDIS" {
        panic!("redis magic fail");
    }
    let rdb_ver = std::str::from_utf8(&buf[5..9])
        .unwrap()
        .parse::<u32>()
        .unwrap();
    let mut aux_map: HashMap<String, String> = HashMap::new();
    let mut redis_map: HashMap<String, RedisValue> = HashMap::new();
    let mut buf = buf[9..].iter().copied();
    loop {
        match buf.next().unwrap() {
            0xFA => {
                // parse aux
                let key = string_decode(&mut buf);
                let value = string_decode(&mut buf);
                eprintln!("[rdb] aux: {} {}", key, value);
                aux_map.insert(key, value);
            }
            0xFB => {
                // parse resizedb, currently unused
                let _htable_size = length_decode(&mut buf);
                let _exp_htable_size = length_decode(&mut buf);
                eprintln!("[rdb] 0xFB resizedb unimplemented");
            }
            0xFC => {
                // parse kv but the expiry in using millisecs
                let mut temp = [0u8; 8];
                fill_buffer(&mut temp, &mut buf);
                let unix_millis = u64::from_be_bytes(temp);
                let (key, value) = parse_kv(&mut buf);
                let expiry = UNIX_EPOCH + Duration::from_millis(unix_millis);
                eprintln!("[rdb] kv: {} {}", key, value);
                redis_map.insert(
                    key,
                    RedisValue {
                        content: value,
                        expiry: Some(expiry),
                    },
                );
            }
            0xFD => {
                // parse kv but the expiry in using secs
                let mut temp = [0u8; 4];
                fill_buffer(&mut temp, &mut buf);
                let unix_secs = u32::from_be_bytes(temp);
                let (key, value) = parse_kv(&mut buf);
                let expiry = UNIX_EPOCH + Duration::from_secs(unix_secs as u64);
                eprintln!("[rdb] kv: {} {}", key, value);
                redis_map.insert(
                    key,
                    RedisValue {
                        content: value,
                        expiry: Some(expiry),
                    },
                );
            }
            0xFE => {
                // selectdb, idk what is this
                eprintln!("[rdb] 0xFE selectdb unimplemented");
                let _db_selection = length_decode(&mut buf);
            }
            0xFF => break,
            _ => {}
        }
    }
    RdbParseResult {
        rdb_ver,
        aux_map,
        redis_map,
    }
}

fn string_decode<T: Iterator<Item = u8>>(buf: &mut T) -> String {
    match length_decode(buf) {
        DecodedLength::ToString(len) => len.to_string(),
        DecodedLength::ReadMore(len) => {
            let strbytes = buf.take(len).collect::<Vec<_>>();
            std::str::from_utf8(&strbytes).unwrap().to_string()
        }
    }
}

fn length_decode<T: Iterator<Item = u8>>(buf: &mut T) -> DecodedLength {
    // extract first 2 bits using 11000000
    let first = buf.next().unwrap();
    match first & 0b11000000 {
        0b11000000 => {
            // read remaining 6 bits to know what to do
            let len = match first & 0b00111111 {
                0 => {
                    // 8 bit int follow
                    buf.next().unwrap() as usize
                }
                1 => {
                    // 16 bit int follow
                    let c = (buf.next().unwrap() as usize) << 8;
                    let d = buf.next().unwrap() as usize;
                    c | d
                }
                2 => {
                    // 32 bit int follow
                    let a = (buf.next().unwrap() as usize) << 24;
                    let b = (buf.next().unwrap() as usize) << 16;
                    let c = (buf.next().unwrap() as usize) << 8;
                    let d = buf.next().unwrap() as usize;
                    a | b | c | d
                }
                _ => {
                    panic!("rdb file is not valid")
                }
            };
            DecodedLength::ToString(len)
        }
        0b10000000 => {
            // discard remaining 6 bits and use the 8 bits from second
            let second = buf.next().unwrap();
            DecodedLength::ReadMore(second as usize)
        }
        0b01000000 => {
            // read additional byte and combined 14 bits is the length
            let second = buf.next().unwrap();
            let len = ((first & 0b00111111) as usize) << 8 | (second as usize);
            DecodedLength::ReadMore(len)
        }
        0b00000000 => {
            // the next 6 bits represent the length
            DecodedLength::ReadMore((first & 0b00111111) as usize)
        }
        _ => unreachable!(),
    }
}

fn parse_kv<T: Iterator<Item = u8>>(buf: &mut T) -> (String, String) {
    let value_type = ValueType::from_u8(buf.next().unwrap());
    let key = string_decode(buf);
    let value = match value_type {
        ValueType::String => string_decode(buf),
        _ => unimplemented!("bro i cant"),
    };
    (key, value)
}

fn fill_buffer<T: Iterator<Item = u8>>(buffer: &mut [u8], it: &mut T) {
    for item in buffer {
        *item = it.next().unwrap();
    }
}
