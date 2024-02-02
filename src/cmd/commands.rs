use std::time::{Duration, SystemTime};

use super::parser::OptionalArgs;
use crate::resp::database::{Database, RedisValue};
use crate::resp::dtypes::{BulkString, Null, Resp, SimpleError, SimpleString};

trait Run {
    fn run(self, db: Database) -> Result<Resp, SimpleError>;
}
// TODO add from_iter trait that signals this trait need to be constructed by bulkstring?
pub enum Cmd {
    Ping(Ping),
    Echo(Echo),
    Set(Set),
    Get(Get),
}
impl Cmd {
    pub fn from_bulk_strings<T>(it: &mut T) -> Result<Cmd, SimpleError>
    where
        T: Iterator<Item = BulkString>,
    {
        let cmd_str = it.next().ok_or(SimpleError("command is empty".into()))?;
        match &cmd_str.0.to_lowercase()[..] {
            // TODO ping can be with 1 param, use option
            "ping" => Ok(Cmd::Ping(Ping)),
            "echo" => Ok(Cmd::Echo(Echo::from_iter(it)?)),
            "set" => Ok(Cmd::Set(Set::from_iter(it)?)),
            "get" => Ok(Cmd::Get(Get::from_iter(it)?)),
            other => Err(SimpleError(format!("ERR command '{other}' not found"))),
        }
    }
    pub fn respond(self, db: Database) -> Result<Resp, SimpleError> {
        match self {
            Cmd::Ping(inner) => inner.run(db),
            Cmd::Echo(inner) => inner.run(db),
            Cmd::Set(inner) => inner.run(db),
            Cmd::Get(inner) => inner.run(db),
        }
    }
}

struct Ping;
impl Run for Ping {
    fn run(self, _: Database) -> Result<Resp, SimpleError> {
        Ok(Resp::SimpleString(SimpleString("PONG".into())))
    }
}

struct Echo(String);
impl Run for Echo {
    fn run(self, _: Database) -> Result<Resp, SimpleError> {
        Ok(Resp::SimpleString(SimpleString(self.0)))
    }
}
impl Echo {
    fn from_iter<T>(it: &mut T) -> Result<Self, SimpleError>
    where
        T: Iterator<Item = BulkString>,
    {
        let param = it
            .next()
            .ok_or(SimpleError("echo param not found".into()))?;
        Ok(Echo(param.0))
    }
}

struct Set {
    key: String,
    value: String,
    expiry: Option<SystemTime>,
}
impl Run for Set {
    fn run(self, db: Database) -> Result<Resp, SimpleError> {
        let mut map = db.write().unwrap();
        map.insert(
            self.key,
            RedisValue {
                content: self.value,
                expiry: self.expiry,
            },
        );
        Ok(Resp::SimpleString(SimpleString("OK".into())))
    }
}
impl Set {
    fn from_iter<T>(it: &mut T) -> Result<Self, SimpleError>
    where
        T: Iterator<Item = BulkString>,
    {
        let key = it.next().ok_or(SimpleError("key param not found".into()))?;
        let value = it
            .next()
            .ok_or(SimpleError("value param not found".into()))?;
        // parse and handle optional arguments
        let mut args = OptionalArgs::new(&["px", "ex"], &[]);
        args.insert_from_iter(&mut it.map(|x| x.0))
            .map_err(|x| SimpleError(format!("SET param {x} is unknown")))?;
        let millis = match (args.args.get("px").unwrap(), args.args.get("ex").unwrap()) {
            (Some(_px), Some(_ex)) => Err(SimpleError("PX and EX are mutually exclusive".into()))?,
            (Some(px), None) => Some(
                px.parse::<u64>()
                    .map_err(|_| SimpleError("PX cannot be parsed".into()))?,
            ),
            (None, Some(ex)) => Some(
                ex.parse::<u64>()
                    .map_err(|_| SimpleError("EX cannot be parsed".into()))?
                    .checked_mul(1000)
                    .ok_or(SimpleError("EX overflows".into()))?,
            ),
            (None, None) => None,
        };
        Ok(Set {
            key: key.0,
            value: value.0,
            expiry: millis.map(|mi| {
                SystemTime::now()
                    .checked_add(Duration::from_millis(mi))
                    .unwrap()
            }),
        })
    }
}

struct Get {
    key: String,
}
impl Run for Get {
    fn run(self, db: Database) -> Result<Resp, SimpleError> {
        let mut map = db.write().unwrap();
        match map.get(&self.key) {
            Some(RedisValue {
                content,
                expiry: None,
            }) => Ok(Resp::SimpleString(SimpleString(content.clone()))),
            Some(RedisValue {
                content,
                expiry: Some(expiry),
            }) => {
                // passive eviction:
                // (expiry)   (now)  ->  if expiry is in the past (elapsed exist), delete and return null
                // (now)   (expiry)  ->  if expiry is in the future (elapsed error), keep and return key
                if expiry.elapsed().is_ok() {
                    // evict
                    map.remove(&self.key);
                    Ok(Resp::Null(Null))
                } else {
                    // keep
                    Ok(Resp::SimpleString(SimpleString(content.clone())))
                }
            }
            None => Ok(Resp::Null(Null)),
        }
    }
}
impl Get {
    fn from_iter<T>(it: &mut T) -> Result<Self, SimpleError>
    where
        T: Iterator<Item = BulkString>,
    {
        let key = it.next().ok_or(SimpleError("key param not found".into()))?;
        Ok(Get { key: key.0 })
    }
}
// TODO SimpleError better error message, add readable source
