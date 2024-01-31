use super::resp;

pub enum Command<'a> {
    Ping,
    Echo(Echo<'a>),
}
impl<'a> Command<'a> {
    pub fn from_bulk_string(bs: &'a resp::BulkString) -> Result<Command<'a>, &'static str> {
        let cmd_name = bs.content()[0].to_lowercase();
        let cmd = match &cmd_name[..] {
            "echo" => Command::Echo(Echo::from_bulk_string(bs)?),
            _ => Err("unsupported command")?,
        };
        Ok(cmd)
    }
    pub fn respond(&self) -> String {
        match self {
            Command::Ping => String::from("PONG"),
            Command::Echo(Echo(echo)) => echo.to_string(),
        }
    }
}

struct Echo<'a>(&'a str);
impl<'a> Echo<'a> {
    fn from_bulk_string(bs: &'a resp::BulkString) -> Result<Echo<'a>, &'static str> {
        let content = bs.content();
        if bs.content().len() != 2 {
            return Err("wrong ECHO parameter count");
        }
        Ok(Echo(content[1]))
    }
}
