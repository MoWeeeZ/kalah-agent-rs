use std::net::TcpStream;

use tungstenite::stream::MaybeTlsStream;
use tungstenite::{connect, Error, WebSocket};

use super::Command;

pub struct Connection {
    websocket: WebSocket<MaybeTlsStream<TcpStream>>,

    next_id: u32,
}

impl Connection {
    pub fn new(url: &str) -> Result<Self, Error> {
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
                println!("< {}", msg);
            }
            msg
        })
    }

    fn write(&mut self, msg: String) {
        #[cfg(debug_assertions)]
        {
            println!("> {}", msg);
        }
        self.websocket.write_message(msg.into()).unwrap()
    }

    pub fn read_command(&mut self) -> Result<Command, Error> {
        self.read().map(|msg| msg.parse().unwrap())
    }

    pub fn write_command(&mut self, cmd: &str, ref_id: Option<u32>) {
        let mut msg = format!("{} ", self.next_id);

        if let Some(id_ref) = ref_id {
            msg += &format!("@{id_ref} ");
        }

        msg += cmd;
        msg += "\r\n";

        self.write(msg);

        self.next_id += 2;
    }
}
