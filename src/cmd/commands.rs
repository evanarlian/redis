use std::time::{Duration, SystemTime};

use super::parser::OptionalArgs;
use crate::db::{database::RedisValue, Database};
use crate::resp::{self as R, Resp};

trait Run {
    fn from_args<T>(it: &mut T) -> Result<Self, R::SimpleError>
    where
        T: Iterator<Item = String>,
        Self: Sized;
    fn run(self, db: Database, config_db: Database) -> Result<Resp, R::SimpleError>;
}

pub enum Cmd {
    Ping(Ping),
    Echo(Echo),
    Set(Set),
    Get(Get),
    ConfigGet(ConfigGet),
}
impl Cmd {
    pub fn from_args<T: Iterator<Item = String>>(it: &mut T) -> Result<Cmd, R::SimpleError> {
        let cmd_str = it.next().ok_or(R::SimpleError("command is empty".into()))?;
        match &cmd_str.to_lowercase()[..] {
            "ping" => Ok(Cmd::Ping(Ping::from_args(it)?)),
            "echo" => Ok(Cmd::Echo(Echo::from_args(it)?)),
            "set" => Ok(Cmd::Set(Set::from_args(it)?)),
            "get" => Ok(Cmd::Get(Get::from_args(it)?)),
            "config" => {
                let subcommand = it
                    .next()
                    .ok_or(R::SimpleError("CONFIG subcommand not found".into()))?;
                match &subcommand.to_lowercase()[..] {
                    "get" => Ok(Cmd::ConfigGet(ConfigGet::from_args(it)?)),
                    other => Err(R::SimpleError(format!(
                        "CONFIG subcommand '{other}' not found"
                    ))),
                }
            }
            other => Err(R::SimpleError(format!("command '{other}' not found"))),
        }
    }
    pub fn run(self, db: Database, config_db: Database) -> Result<Resp, R::SimpleError> {
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
    fn from_args<T: Iterator<Item = String>>(it: &mut T) -> Result<Self, R::SimpleError> {
        let (param1, param2) = (it.next(), it.next());
        match (param1, param2) {
            (Some(_), Some(_)) => {
                Err(R::SimpleError("PING only takes 0 or 1 arg".into()))
            }
            (Some(param1), None) => Ok(Ping(param1)),
            (None, Some(_)) => unreachable!(),
            (None, None) => Ok(Ping("PONG".into())),
        }
    }
    fn run(self, _: Database, _: Database) -> Result<Resp, R::SimpleError> {
        Ok(Resp::SimpleString(R::SimpleString(self.0)))
    }
}

struct Echo(String);
impl Run for Echo {
    fn from_args<T: Iterator<Item = String>>(it: &mut T) -> Result<Self, R::SimpleError> {
        let param = it
            .next()
            .ok_or(R::SimpleError("ECHO param not found".into()))?;
        Ok(Echo(param))
    }
    fn run(self, _: Database, _: Database) -> Result<Resp, R::SimpleError> {
        Ok(Resp::SimpleString(R::SimpleString(self.0)))
    }
}

struct Set {
    key: String,
    value: String,
    expiry: Option<SystemTime>,
}
impl Run for Set {
    fn from_args<T: Iterator<Item = String>>(it: &mut T) -> Result<Self, R::SimpleError> {
        let key = it
            .next()
            .ok_or(R::SimpleError("SET arg key not found".into()))?;
        let value = it
            .next()
            .ok_or(R::SimpleError("SET arg value not found".into()))?;
        // parse and handle optional arguments
        let mut args = OptionalArgs::new(&["px", "ex"], &[]);
        args.insert_from_iter(it)
            .map_err(|x| R::SimpleError(format!("SET arg {x} is unknown")))?;
        let millis = match (args.args.get("px").unwrap(), args.args.get("ex").unwrap()) {
            (Some(_px), Some(_ex)) => Err(R::SimpleError(
                "SET PX and EX are mutually exclusive".into(),
            ))?,
            (Some(px), None) => Some(
                px.parse::<u64>()
                    .map_err(|_| R::SimpleError("SET PX cannot be parsed".into()))?,
            ),
            (None, Some(ex)) => Some(
                ex.parse::<u64>()
                    .map_err(|_| R::SimpleError("SET EX cannot be parsed".into()))?
                    .checked_mul(1000)
                    .ok_or(R::SimpleError("SET EX overflows".into()))?,
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
    fn run(self, db: Database, _: Database) -> Result<Resp, R::SimpleError> {
        let mut guard = db.write().unwrap();
        guard.set(
            self.key,
            RedisValue {
                content: self.value,
                expiry: self.expiry,
            },
        );
        Ok(Resp::SimpleString(R::SimpleString("OK".into())))
    }
}

struct Get {
    key: String,
}
impl Run for Get {
    fn from_args<T: Iterator<Item = String>>(it: &mut T) -> Result<Self, R::SimpleError> {
        let key = it
            .next()
            .ok_or(R::SimpleError("GET arg key not found".into()))?;
        Ok(Get { key })
    }
    fn run(self, db: Database, _: Database) -> Result<Resp, R::SimpleError> {
        let mut guard = db.write().unwrap();
        match guard.get(&self.key) {
            Some(RedisValue { content, expiry: _ }) => {
                Ok(Resp::SimpleString(R::SimpleString(content)))
            }
            None => Ok(Resp::Null(R::Null)),
        }
    }
}

struct ConfigGet {
    keys: Vec<String>,
}
impl Run for ConfigGet {
    fn from_args<T: Iterator<Item = String>>(it: &mut T) -> Result<Self, R::SimpleError> {
        let keys = it.collect::<Vec<_>>();
        if keys.is_empty() {
            return Err(R::SimpleError(
                "CONFIG GET key args must be at least 1".into(),
            ));
        }
        Ok(ConfigGet { keys })
    }
    fn run(self, _: Database, config_db: Database) -> Result<Resp, R::SimpleError> {
        let mut guard = config_db.write().unwrap();
        // sort + dedup is basically unique
        let mut keys = self.keys;
        keys.sort();
        keys.dedup();
        let mut array_content = vec![];
        for key in keys {
            if let Some(RedisValue { content, expiry: _ }) = guard.get(&key) {
                array_content.push(Resp::BulkString(R::BulkString(key)));
                array_content.push(Resp::BulkString(R::BulkString(content)));
            }
        }
        Ok(Resp::Array(R::Array(array_content)))
    }
}
