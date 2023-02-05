use std::io::{Read, Write};
use std::net::TcpStream;

// use tungstenite::stream::MaybeTlsStream;
// use tungstenite::{connect, WebSocket};

use super::Command;

#[derive(Debug)]
enum Stream {
    // Websocket(WebSocket<MaybeTlsStream<TcpStream>>),
    TcpStream { stream: TcpStream, buf: String },
}

pub struct Connection {
    stream: Stream,

    next_id: u32,
}

impl Connection {
    /* #[allow(dead_code)]
    pub fn new_websocket(url: &str) -> Result<Self, String> {
        match connect(url) {
            Ok((mut websocket, _)) => {
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

                let stream = Stream::Websocket(websocket);

                Ok(Connection { stream, next_id: 1 })
            }
            Err(err) => Err(err.to_string()),
        }
    } */

    #[allow(dead_code)]
    pub fn new_tcpstream(url: &str) -> Result<Self, std::io::Error> {
        TcpStream::connect(url).map(|stream| {
            stream.set_nonblocking(true).unwrap();

            let stream = Stream::TcpStream {
                stream,
                buf: String::new(),
            };

            Connection { stream, next_id: 1 }
        })
    }

    fn read(&mut self) -> Option<String> {
        match self.stream {
            /* Stream::Websocket(ref mut websocket) => match websocket.read_message() {
                Ok(msg) => Some(msg.into_text().unwrap()),
                Err(tungstenite::Error::Io(err)) if err.kind() == std::io::ErrorKind::WouldBlock => None,
                Err(err) => panic!("Error while reading from Websocket stream: {err}"),
            }, */
            Stream::TcpStream {
                ref mut stream,
                ref mut buf,
            } => {
                let mut read_buf = [0; 1024];

                match stream.read(&mut read_buf) {
                    Ok(len) if len > 0 => {
                        // Some(std::str::from_utf8(&read_buf[0..len]).unwrap().to_owned())
                        *buf += std::str::from_utf8(&read_buf[0..len]).unwrap();

                        println!("New buf: \"{buf}\"");
                    }
                    Ok(0) => {
                        println!("Connection closed, exiting");
                        std::process::exit(0);
                    }
                    Ok(_) => unreachable!(),
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(err) => {
                        panic!("Error while reading from TcpStream: {err}");
                    }
                };

                if let Some(idx) = buf.find('\n') {
                    let buf_rest = buf.split_off(idx + 1);
                    let msg = std::mem::replace(buf, buf_rest);

                    println!("Split \"{msg}\" from buf");
                    println!("Buf contains \"{buf}\"");

                    if !msg.is_empty() {
                        Some(msg)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        }
        .map(|msg| {
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

        match self.stream {
            // Stream::Websocket(ref mut websocket) => websocket.write_message(msg.into()).unwrap(),
            Stream::TcpStream { ref mut stream, buf: _ } => {
                stream.write_all(msg.as_bytes()).unwrap();
            }
        }
    }

    pub fn read_command(&mut self) -> Option<Command> {
        self.read().map(|msg| msg.parse().unwrap())
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
