use std::io;
use std::mem::swap;
use std::net::SocketAddr;
use std::collections::VecDeque;

use mio::tcp::TcpStream;
use mio::{TryWrite, TryRead};

use super::messages::{RawMessage, MessageBuffer, HEADER_SIZE};

pub struct IncomingConnection {
    socket: TcpStream,
    address: SocketAddr,
    raw: MessageBuffer,
    position: usize,
}

pub struct OutgoingConnection {
    socket: TcpStream,
    address: SocketAddr,
    queue: VecDeque<RawMessage>,
    position: usize,
}

impl IncomingConnection {
    pub fn new(socket: TcpStream, address: SocketAddr)
            -> IncomingConnection {
        IncomingConnection {
            socket: socket,
            address: address,
            raw: MessageBuffer::empty(),
            position: 0,
        }
    }

    pub fn socket(&self) -> &TcpStream {
        &self.socket
    }

    pub fn address(&self) -> &SocketAddr {
        &self.address
    }

    fn read(&mut self) -> io::Result<Option<usize>> {
        // FIXME: we shouldn't read more than HEADER_SIZE or total_length()
        // TODO: read into growable Vec, not into [u8]
        if self.position == HEADER_SIZE &&
           self.raw.actual_length() == HEADER_SIZE {
            self.raw.allocate_payload();
        }
        let buffer : &mut [u8] = self.raw.as_mut();
        self.socket.try_read(&mut buffer[self.position..])
    }

    pub fn readable(&mut self) -> io::Result<Option<MessageBuffer>> {
        // TODO: raw length == 0?
        // TODO: maximum raw length?
        loop {
            match self.read()? {
                None | Some(0) => return Ok(None),
                Some(n) => {
                    self.position += n;
                    if self.position >= HEADER_SIZE &&
                       self.position == self.raw.total_length() {
                        let mut raw = MessageBuffer::empty();
                        swap(&mut raw, &mut self.raw);
                        self.position = 0;
                        return Ok(Some(raw))
                    }
                }
            }
        }
    }
}

impl OutgoingConnection {
    pub fn new(socket: TcpStream, address: SocketAddr)
            -> OutgoingConnection {
        OutgoingConnection {
            socket: socket,
            address: address,
            queue: VecDeque::new(),
            position: 0
        }
    }

    pub fn socket(&self) -> &TcpStream {
        &self.socket
    }

    pub fn address(&self) -> &SocketAddr {
        &self.address
    }

    pub fn writable(&mut self) -> io::Result<()> {
        // TODO: use try_write_buf
        while let Some(message) = self.queue.pop_front() {
            match self.socket.try_write(message.as_ref().as_ref())? {
                None | Some(0) => {
                    self.queue.push_front(message);
                    break
                },
                Some(n) => {
                    // FIXME: What if we write less than message size?
                    self.position += n;
                    if n == message.actual_length() {
                        self.position = 0;
                    }
                }
            }
        }
        // TODO: reregister
        return Ok(())
    }

    pub fn send(&mut self, message: RawMessage) {
        // TODO: capacity overflow
        // TODO: reregister
        self.queue.push_back(message);
    }

    pub fn is_idle(&self) -> bool {
        self.queue.is_empty()
    }
}
