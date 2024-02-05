use std::time::{Duration, SystemTime};

use super::parser::OptionalArgs;
use crate::db::{database::RedisValue, Database};
use crate::resp::dtypes::{Array, BulkString, Null, Resp, SimpleError, SimpleString};

trait Run {
    fn run(self, db: Database, config_db: Database) -> Result<Resp, SimpleError>;
}
// TODO add from_iter trait that signals this trait need to be constructed by bulkstring?
pub enum Cmd {
    Ping(Ping),
    Echo(Echo),
    Set(Set),
    Get(Get),
    ConfigGet(ConfigGet),
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
            "config" => {
                let subcommand = it.next().ok_or(SimpleError("subcommand is empty".into()))?;
                match &subcommand.0.to_lowercase()[..] {
                    "get" => Ok(Cmd::ConfigGet(ConfigGet::from_iter(it)?)),
                    other => Err(SimpleError(format!("ERR command '{other}' not found"))),
                }
            }
            other => Err(SimpleError(format!("ERR command '{other}' not found"))),
        }
    }
    pub fn respond(self, db: Database, config_db: Database) -> Result<Resp, SimpleError> {
        // tbh this design is so bad because every command now get both the db and config_db
        match self {
            Cmd::Ping(inner) => inner.run(db, config_db),
            Cmd::Echo(inner) => inner.run(db, config_db),
            Cmd::Set(inner) => inner.run(db, config_db),
            Cmd::Get(inner) => inner.run(db, config_db),
            Cmd::ConfigGet(inner) => inner.run(db, config_db),
        }
    }
}

struct Ping;
impl Run for Ping {
    fn run(self, _: Database, _: Database) -> Result<Resp, SimpleError> {
        Ok(Resp::SimpleString(SimpleString("PONG".into())))
    }
}

struct Echo(String);
impl Run for Echo {
    fn run(self, _: Database, _: Database) -> Result<Resp, SimpleError> {
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
    fn run(self, db: Database, _: Database) -> Result<Resp, SimpleError> {
        let mut guard = db.write().unwrap();
        guard.set(
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
    fn run(self, db: Database, _: Database) -> Result<Resp, SimpleError> {
        let mut guard = db.write().unwrap();
        match guard.get(&self.key) {
            Some(RedisValue { content, expiry: _ }) => {
                Ok(Resp::SimpleString(SimpleString(content)))
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

struct ConfigGet {
    keys: Vec<String>,
}
impl Run for ConfigGet {
    fn run(self, _: Database, config_db: Database) -> Result<Resp, SimpleError> {
        let mut guard = config_db.write().unwrap();
        // sort + dedup is basically unique
        let mut keys = self.keys;
        keys.sort();
        keys.dedup();
        let mut array_content = vec![];
        for key in keys {
            if let Some(RedisValue { content, expiry: _ }) = guard.get(&key) {
                array_content.push(Resp::BulkString(BulkString(key)));
                array_content.push(Resp::BulkString(BulkString(content)));
            }
        }
        Ok(Resp::Array(Array(array_content)))
    }
}
impl ConfigGet {
    fn from_iter<T>(it: &mut T) -> Result<Self, SimpleError>
    where
        T: Iterator<Item = BulkString>,
    {
        let keys = it.map(|bs| bs.0).collect::<Vec<_>>();
        if keys.is_empty() {
            return Err(SimpleError("config get key args must be at least 1".into()));
        }
        Ok(ConfigGet { keys })
    }
}

// TODO SimpleError better error message, add readable source
// TODO iterator on bulkstring is bad or not?? it is only for getting (.0) anyway
