use crate::resp::dtypes::Null;

use super::resp::dtypes::{BulkString, Resp, SimpleError, SimpleString};
use super::server::Database;

trait Run {
    // for potentially write actions, can return error
    // example: GET command is mutable, because it might expire the key
    fn run(self, db: Database) -> Result<Resp, SimpleError>;
}
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
}
impl Run for Set {
    fn run(self, db: Database) -> Result<Resp, SimpleError> {
        let mut map = db.write().unwrap();
        map.insert(self.key, self.value);
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
        Ok(Set {
            key: key.0,
            value: value.0,
        })
    }
}

struct Get(String);
impl Run for Get {
    fn run(self, db: Database) -> Result<Resp, SimpleError> {
        let map = db.read().unwrap();
        match map.get(&self.0) {
            Some(v) => Ok(Resp::SimpleString(SimpleString(v.clone()))),
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
        Ok(Get(key.0))
    }
}
// TODO should i use Self or Set? Standardize!
// TODO change respond to run and runmut?
// TODO fix naming from errors, super bad lol, eg key param not found, well on what?>
