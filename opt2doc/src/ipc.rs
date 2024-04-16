use std::{
    collections::BTreeMap,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use interprocess::local_socket::{LocalSocketListener, LocalSocketStream};

use crate::DocOpts;

pub struct DocClientState {
    conn: Option<LocalSocketStream>,
    opt: DocOpts
}

pub struct DocServerState {
    listener: LocalSocketListener,
    max_id: usize,
    streams: BTreeMap<usize, LocalSocketStream>,
}

impl DocServerState {
    /// create non-blocking socket listener
    ///
    /// panic when fail
    pub fn new(name: PathBuf) -> Self {
        let listener = LocalSocketListener::bind(name).unwrap();
        listener.set_nonblocking(true).unwrap();
        Self {
            listener,
            max_id: 0,
            streams: Default::default(),
        }
    }

    /// Try to accept a new connection
    ///
    /// will return None if no connection is available
    /// and never block
    pub fn try_accept(&self) -> Option<LocalSocketStream> {
        match self.listener.accept() {
            Ok(connection) => {
                connection.set_nonblocking(true).unwrap();
                Some(connection)
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => None,
            Err(error) => panic!("Incoming connection failed: {}", error),
        }
    }
}
