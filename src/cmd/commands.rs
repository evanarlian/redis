use std::time::{Duration, SystemTime};

use super::parser::OptionalArgs;
use crate::db::{database::RedisValue, Database};
use crate::resp::dtypes::{self as D, Resp};

trait Run {
    fn from_args<T>(it: &mut T) -> Result<Self, D::SimpleError>
    where
        T: Iterator<Item = String>,
        Self: Sized;
    fn run(self, db: Database, config_db: Database) -> Result<Resp, D::SimpleError>;
}

pub enum Cmd {
    Ping(Ping),
    Echo(Echo),
    Set(Set),
    Get(Get),
    ConfigGet(ConfigGet),
}
impl Cmd {
    pub fn from_args<T: Iterator<Item = String>>(it: &mut T) -> Result<Cmd, D::SimpleError> {
        let cmd_str = it.next().ok_or(D::SimpleError("command is empty".into()))?;
        match &cmd_str.to_lowercase()[..] {
            "ping" => Ok(Cmd::Ping(Ping::from_args(it)?)),
            "echo" => Ok(Cmd::Echo(Echo::from_args(it)?)),
            "set" => Ok(Cmd::Set(Set::from_args(it)?)),
            "get" => Ok(Cmd::Get(Get::from_args(it)?)),
            "config" => {
                let subcommand = it
                    .next()
                    .ok_or(D::SimpleError("CONFIG subcommand not found".into()))?;
                match &subcommand.to_lowercase()[..] {
                    "get" => Ok(Cmd::ConfigGet(ConfigGet::from_args(it)?)),
                    other => Err(D::SimpleError(format!("CONFIG subcommand '{other}' not found"))),
                }
            }
            other => Err(D::SimpleError(format!("command '{other}' not found"))),
        }
    }
    pub fn run(self, db: Database, config_db: Database) -> Result<Resp, D::SimpleError> {
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

struct Ping(String);
impl Run for Ping {
    fn from_args<T: Iterator<Item = String>>(it: &mut T) -> Result<Self, D::SimpleError> {
        let (param1, param2) = (it.next(), it.next());
        match (param1, param2) {
            (Some(param1), Some(param2)) => {
                Err(D::SimpleError("PING only takes 0 or 1 arg".into()))
            }
            (Some(param1), None) => Ok(Ping(param1)),
            (None, Some(param2)) => unreachable!(),
            (None, None) => Ok(Ping("PONG".into())),
        }
    }
    fn run(self, _: Database, _: Database) -> Result<Resp, D::SimpleError> {
        Ok(Resp::SimpleString(D::SimpleString(self.0)))
    }
}

struct Echo(String);
impl Run for Echo {
    fn from_args<T: Iterator<Item = String>>(it: &mut T) -> Result<Self, D::SimpleError> {
        let param = it
            .next()
            .ok_or(D::SimpleError("ECHO param not found".into()))?;
        Ok(Echo(param))
    }
    fn run(self, _: Database, _: Database) -> Result<Resp, D::SimpleError> {
        Ok(Resp::SimpleString(D::SimpleString(self.0)))
    }
}

struct Set {
    key: String,
    value: String,
    expiry: Option<SystemTime>,
}
impl Run for Set {
    fn from_args<T: Iterator<Item = String>>(it: &mut T) -> Result<Self, D::SimpleError> {
        let key = it
            .next()
            .ok_or(D::SimpleError("SET arg key not found".into()))?;
        let value = it
            .next()
            .ok_or(D::SimpleError("SET arg value not found".into()))?;
        // parse and handle optional arguments
        let mut args = OptionalArgs::new(&["px", "ex"], &[]);
        args.insert_from_iter(it)
            .map_err(|x| D::SimpleError(format!("SET arg {x} is unknown")))?;
        let millis = match (args.args.get("px").unwrap(), args.args.get("ex").unwrap()) {
            (Some(_px), Some(_ex)) => Err(D::SimpleError(
                "SET PX and EX are mutually exclusive".into(),
            ))?,
            (Some(px), None) => Some(
                px.parse::<u64>()
                    .map_err(|_| D::SimpleError("SET PX cannot be parsed".into()))?,
            ),
            (None, Some(ex)) => Some(
                ex.parse::<u64>()
                    .map_err(|_| D::SimpleError("SET EX cannot be parsed".into()))?
                    .checked_mul(1000)
                    .ok_or(D::SimpleError("SET EX overflows".into()))?,
            ),
            (None, None) => None,
        };
        let expiry = millis.map(|mi| {
            SystemTime::now()
                .checked_add(Duration::from_millis(mi))
                .unwrap()
        });
        Ok(Set { key, value, expiry })
    }
    fn run(self, db: Database, _: Database) -> Result<Resp, D::SimpleError> {
        let mut guard = db.write().unwrap();
        guard.set(
            self.key,
            RedisValue {
                content: self.value,
                expiry: self.expiry,
            },
        );
        Ok(Resp::SimpleString(D::SimpleString("OK".into())))
    }
}

struct Get {
    key: String,
}
impl Run for Get {
    fn from_args<T: Iterator<Item = String>>(it: &mut T) -> Result<Self, D::SimpleError> {
        let key = it
            .next()
            .ok_or(D::SimpleError("GET arg key not found".into()))?;
        Ok(Get { key })
    }
    fn run(self, db: Database, _: Database) -> Result<Resp, D::SimpleError> {
        let mut guard = db.write().unwrap();
        match guard.get(&self.key) {
            Some(RedisValue { content, expiry: _ }) => {
                Ok(Resp::SimpleString(D::SimpleString(content)))
            }
            None => Ok(Resp::Null(D::Null)),
        }
    }
}

struct ConfigGet {
    keys: Vec<String>,
}
impl Run for ConfigGet {
    fn from_args<T: Iterator<Item = String>>(it: &mut T) -> Result<Self, D::SimpleError> {
        let keys = it.collect::<Vec<_>>();
        if keys.is_empty() {
            return Err(D::SimpleError(
                "CONFIG GET key args must be at least 1".into(),
            ));
        }
        Ok(ConfigGet { keys })
    }
    fn run(self, _: Database, config_db: Database) -> Result<Resp, D::SimpleError> {
        let mut guard = config_db.write().unwrap();
        // sort + dedup is basically unique
        let mut keys = self.keys;
        keys.sort();
        keys.dedup();
        let mut array_content = vec![];
        for key in keys {
            if let Some(RedisValue { content, expiry: _ }) = guard.get(&key) {
                array_content.push(Resp::BulkString(D::BulkString(key)));
                array_content.push(Resp::BulkString(D::BulkString(content)));
            }
        }
        Ok(Resp::Array(D::Array(array_content)))
    }
}
