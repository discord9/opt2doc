use std::{
    collections::BTreeMap,
    io::{ErrorKind, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::PathBuf,
};

use interprocess::local_socket::prelude::*;

use crate::{CompsiteMetadata, DocOpts};

/// End of a entire json packet this is ascii's EOT(End of Transmission) control character
/// which should not appear in a json string
pub const END_OF_PACKET: &str = "\u{0004}";

/// Socket use this env var to determine the url to connect to/ bind to
pub const URL_ENV_VAR_NAME: &str = "OPT2DOC_URL";

/// Default url to bind to
pub const DEFAULT_URL: &str = "127.0.0.1:41503";

pub fn get_socket_url() -> String {
    if let Ok(url) = std::env::var(URL_ENV_VAR_NAME) {
        url
    } else {
        DEFAULT_URL.to_string()
    }
}

pub struct DocClientState {
    conn: Option<TcpStream>,
}

impl Default for DocClientState {
    fn default() -> Self {
        Self::new()
    }
}

impl DocClientState {
    /// Try to read options and connect to the server, if failed, further `send` call is ignored
    pub fn new() -> Self {
        let conn = {
            // TODO: maybe diagnostic warnning
            let path = get_socket_url();
            match TcpStream::connect(path) {
                Ok(conn) => {
                    conn.set_nonblocking(true).unwrap();
                    Some(conn)
                }
                Err(e) => {
                    println!("WARN fail to connect socket: {:?}", e);
                    None
                }
            }
        };
        DocClientState { conn }
    }
    /// if the connection is made
    pub fn is_connected(&self) -> bool {
        self.conn.is_some()
    }

    pub fn try_send(&mut self, msg: String) {
        if let Some(ref mut conn) = self.conn {
            conn.write_all(msg.as_bytes()).unwrap();
            conn.write_all(END_OF_PACKET.as_bytes()).unwrap();
        }
    }

    /// send a new type to the server
    pub fn try_insert_type(&mut self, compsite: CompsiteMetadata) {
        let out_str = serde_json::to_string_pretty(&compsite).unwrap();
        self.try_send(format!("{}\n", out_str));
    }
}

pub struct DocServerState {
    listener: TcpListener,
    max_id: usize,
    streams: BTreeMap<usize, (TcpStream, SocketAddr)>,
}

impl DocServerState {
    /// create non-blocking socket listener
    ///
    /// panic when fail
    pub fn new(name: &str) -> Self {
        let listener = TcpListener::bind(name).unwrap();
        listener.set_nonblocking(true).unwrap();
        Self {
            listener,
            max_id: 0,
            streams: Default::default(),
        }
    }

    /// Try to accept a new connection, return the id of that connection
    ///
    /// will return None if no connection is available
    /// and never block
    pub fn try_accept(&mut self) -> Option<usize> {
        match self.listener.accept() {
            Ok((stream, addr)) => {
                stream.set_nonblocking(true).unwrap();
                let new_id = self.max_id;
                self.max_id += 1;
                self.streams.insert(new_id, (stream, addr));
                Some(new_id)
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => None,
            Err(error) => panic!("Incoming connection failed: {}", error),
        }
    }

    /// Iterate over all the messages received from all the connections
    ///
    /// non-blocking, and will only collect
    /// TODO: error handling
    pub fn try_recv(&mut self) -> Vec<CompsiteMetadata> {
        let mut ret = vec![];
        for (idx, stream) in self.streams.iter_mut() {
            let mut all_avail = String::new();
            loop {
                let mut buf = [0; 1024];
                match stream.0.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => all_avail.push_str(&String::from_utf8_lossy(&buf[..n])),
                    Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                    Err(e) => panic!("Error reading from connection {}: {:?}", idx, e),
                }
            }
            // split all_avail by `\n` and remove empty lines
            let all_avail = all_avail
                .split(END_OF_PACKET)
                .filter(|s| !s.is_empty())
                .map(|s| serde_json::from_str(s).unwrap());
            ret.extend(all_avail);
        }
        ret
    }
}
