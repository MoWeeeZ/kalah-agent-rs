use std::net::TcpStream;

use tungstenite::stream::MaybeTlsStream;
use tungstenite::{connect, Error, WebSocket};
use url::Url;

use super::Command;

pub struct Connection {
    websocket: WebSocket<MaybeTlsStream<TcpStream>>,

    next_id: u32,
}

impl Connection {
    pub fn new(url: &Url) -> Result<Self, Error> {
        connect(url).map(|(mut websocket, _)| {
            match websocket.get_mut() {
                MaybeTlsStream::Plain(s) => s
                    .set_nonblocking(true)
                    .expect("Could not set TlsStream to non-blocking"),
                MaybeTlsStream::NativeTls(s) => s
                    .get_mut()
                    .set_nonblocking(true)
                    .expect("Could not set TlsStream to non-blocking"),
                _ => panic!("Unknown"),
            };
            Connection { websocket, next_id: 1 }
        })
    }

    fn read(&mut self) -> Result<String, Error> {
        self.websocket.read_message().map(|msg| {
            let msg = msg.into_text().unwrap();
            #[cfg(debug_assertions)]
            {
                println!("< {msg}");
            }
            msg
        })
    }

    fn write(&mut self, msg: String) {
        #[cfg(debug_assertions)]
        {
            println!("> {msg}");
        }
        self.websocket.write_message(msg.into()).unwrap()
    }

    pub fn read_command(&mut self) -> Option<Command> {
        match self.read().map(|msg| msg.parse().unwrap()) {
            Ok(cmd) => Some(cmd),
            Err(tungstenite::Error::Io(_)) => None,
            Err(err) => panic!("Error reading command: {err}"),
        }
    }

    pub fn write_command(&mut self, cmd: &str, ref_id: Option<u32>) {
        let mut msg = if let Some(id_ref) = ref_id {
            format!("{}@{} ", self.next_id, id_ref)
        } else {
            format!("{} ", self.next_id)
        };

        msg += cmd;
        msg += "\r\n";

        self.write(msg);

        self.next_id += 2;
    }
}
