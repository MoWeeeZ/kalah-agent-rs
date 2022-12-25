use std::fmt::Display;
use std::str::FromStr;

use lazy_static::lazy_static;
use regex::Regex;

use crate::Board;

#[derive(Debug)]
pub enum Command {
    Kpg {
        id: Option<u32>,
        ref_id: Option<u32>,
        major: u8,
        minor: u8,
        patch: u8,
    },
    State {
        id: Option<u32>,
        ref_id: Option<u32>,
        board: Board,
    },
    Stop {
        id: Option<u32>,
        ref_id: Option<u32>,
    },
    Ok {
        id: Option<u32>,
        ref_id: Option<u32>,
    },
    Ping {
        id: Option<u32>,
        ref_id: Option<u32>,
        msg: String,
    },
    Pong {
        id: Option<u32>,
        ref_id: Option<u32>,
        msg: String,
    },
    Goodbye {
        id: Option<u32>,
        ref_id: Option<u32>,
    },
    Error {
        id: Option<u32>,
        ref_id: Option<u32>,
        msg: String,
    },
}

// from kalah-game/client/pykgp/kgp.py
lazy_static! {
    static ref COMMAND_REGEX: Regex = Regex::new(
        &r"
^                   
\s*                 
(?:                 
    (?P<id>\d+)         
    (?:@(?P<ref>\d+))?  
    \s+                 
)?
(?P<cmd>\w+)       
(?:                 
    \s+                 
    (?P<args>.*?)       
)?
\s*
$                
".chars().filter(|c| !c.is_whitespace()).collect::<String>(),
    )
    .unwrap();
    // static ref KGP_REGEX: Regex = Regex::new(r"^(?P<major>\d+)\s*(?P<minor>\d+)\s*(?P<patch>\d+)\s*$").unwrap();
}

// based on connect in kalah-game/client/pykgp/kgp.py
impl FromStr for Command {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let command_captures = COMMAND_REGEX
            .captures(s)
            .ok_or(format!("command \"{}\" didn't match regex", s))?;

        let id = command_captures
            .name("id")
            .map(|capture| capture.as_str().parse::<u32>())
            .transpose()
            .map_err(|_| "Could not parse id")?;

        let id_ref = command_captures
            .name("ref")
            .map(|capture| capture.as_str().parse::<u32>())
            .transpose()
            .map_err(|_| "Could not parse ref")?;

        // non-optional: if it didn't match, we already returned an Err earlier
        let cmd = command_captures.name("cmd").unwrap().as_str();

        let args = command_captures
            .name("args")
            .map(|cap: regex::Match| cap.as_str())
            .unwrap_or("");

        match cmd {
            "kgp" => {
                let args_vec: Vec<&str> = args.split_ascii_whitespace().collect();

                if args_vec.len() != 3 {
                    return Err(format!("Unexpected args for kpg command: \"{}\"", args));
                }

                let major: u8 = args_vec[0]
                    .parse()
                    .map_err(|_| "Could not parse major version of kpg command")?;
                let minor: u8 = args_vec[1]
                    .parse()
                    .map_err(|_| "Could not parse minor version of kpg command")?;
                let patch: u8 = args_vec[2]
                    .parse()
                    .map_err(|_| "Could not parse patch version of kpg command")?;

                Ok(Command::Kpg {
                    id,
                    ref_id: id_ref,
                    major,
                    minor,
                    patch,
                })
            }
            "state" => {
                let args_vec: Vec<&str> = args.split_ascii_whitespace().collect();

                if args_vec.len() != 1 {
                    return Err(format!("Unexpected args for state command: \"{}\"", args));
                }

                let board = Board::from_kpg(args_vec[0]);

                Ok(Command::State {
                    id,
                    ref_id: id_ref,
                    board,
                })
            }
            "stop" => Ok(Command::Stop { id, ref_id: id_ref }),
            "ok" => Ok(Command::Ok { id, ref_id: id_ref }),
            "ping" => Ok(Command::Ping {
                id,
                ref_id: id_ref,
                msg: args.to_owned(),
            }),
            "pong" => Ok(Command::Pong {
                id,
                ref_id: id_ref,
                msg: args.to_owned(),
            }),
            "goodbye" => Ok(Command::Goodbye { id, ref_id: id_ref }),
            "error" => Ok(Command::Error {
                id,
                ref_id: id_ref,
                msg: args.to_owned(),
            }),
            _ => Err(format!("Unknown command {}", cmd)),
        }
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::Kpg {
                id,
                ref_id,
                major,
                minor,
                patch,
            } => {
                if let Some(id) = id {
                    write!(f, "{}", id)?;
                }
                if let Some(ref_id) = ref_id {
                    write!(f, "@{}", ref_id)?;
                }
                write!(f, " kpg {} {} {}", major, minor, patch)
            }
            Command::State { id, ref_id, board } => {
                if let Some(id) = id {
                    write!(f, "{}", id)?;
                }
                if let Some(ref_id) = ref_id {
                    write!(f, "@{}", ref_id)?;
                }
                write!(f, " state {}", board.to_kgp())
            }
            Command::Stop { id, ref_id } => {
                if let Some(id) = id {
                    write!(f, "{}", id)?;
                }
                if let Some(ref_id) = ref_id {
                    write!(f, "@{}", ref_id)?;
                }
                write!(f, " stop")
            }
            Command::Ok { id, ref_id } => {
                if let Some(id) = id {
                    write!(f, "{}", id)?;
                }
                if let Some(ref_id) = ref_id {
                    write!(f, "@{}", ref_id)?;
                }
                write!(f, " ok")
            }
            Command::Ping { id, ref_id, msg } => {
                if let Some(id) = id {
                    write!(f, "{}", id)?;
                }
                if let Some(ref_id) = ref_id {
                    write!(f, "@{}", ref_id)?;
                }
                write!(f, " ping {}", msg)
            }
            Command::Pong { id, ref_id, msg } => {
                if let Some(id) = id {
                    write!(f, "{}", id)?;
                }
                if let Some(ref_id) = ref_id {
                    write!(f, "@{}", ref_id)?;
                }
                write!(f, " pong {}", msg)
            }
            Command::Goodbye { id, ref_id } => {
                if let Some(id) = id {
                    write!(f, "{}", id)?;
                }
                if let Some(ref_id) = ref_id {
                    write!(f, "@{}", ref_id)?;
                }
                write!(f, " goodbye")
            }
            Command::Error { id, ref_id, msg } => {
                if let Some(id) = id {
                    write!(f, "{}", id)?;
                }
                if let Some(ref_id) = ref_id {
                    write!(f, "@{}", ref_id)?;
                }
                write!(f, " error {}", msg)
            }
        }
    }
}
