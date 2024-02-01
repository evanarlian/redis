use super::resp::dtypes::{BulkString, Resp, SimpleError, SimpleString};

trait Command {
    fn respond(self) -> Resp;
}
pub enum Cmd {
    Ping(Ping),
    Echo(Echo),
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
            other => Err(SimpleError(format!("ERR command '{other}' not found"))),
        }
    }
    pub fn respond(self) -> Resp {
        match self {
            Cmd::Ping(inner) => inner.respond(),
            Cmd::Echo(inner) => inner.respond(),
        }
    }
}

struct Ping;
impl Command for Ping {
    fn respond(self) -> Resp {
        Resp::SimpleString(SimpleString("PONG".into()))
    }
}

struct Echo(String);
impl Command for Echo {
    fn respond(self) -> Resp {
        Resp::SimpleString(SimpleString(self.0))
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
