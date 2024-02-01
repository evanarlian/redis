use super::resp::dtypes::{Resp, SimpleString, BulkString};

// trait Command {
//     fn respond() -> Resp<'a>;
// }
// pub enum Cmd<'a> {
//     Ping,
//     Echo(Echo<'a>),
// }

// struct Ping;
// impl Command for Ping {
//     fn respond<'a>() -> Resp<'a> {
//         Resp::SimpleString(SimpleString("PONG"))
//     }
// }


// struct Echo<'a>(&'a str);
// impl <'a>Command for Echo<'a> {
//     fn respond() -> Resp<'a> {
//         Resp::SimpleString(SimpleString("PONG"))
//     }
// }
