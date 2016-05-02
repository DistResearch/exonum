use std::{io, net, mem, collections};

use mio;
use mio::{TryWrite, TryRead};

use super::message::{Message, MessageData, HEADER_SIZE};

pub struct IncomingConnection {
    socket: mio::tcp::TcpStream,
    address: net::SocketAddr,
    data: MessageData,
    position: usize,
}

pub struct OutgoingConnection {
    socket: mio::tcp::TcpStream,
    address: net::SocketAddr,
    queue: collections::VecDeque<Message>,
    position: usize,
}

impl IncomingConnection {
    pub fn new(socket: mio::tcp::TcpStream, address: net::SocketAddr)
            -> IncomingConnection {
        IncomingConnection {
            socket: socket,
            address: address,
            data: MessageData::new(),
            position: 0,
        }
    }

    pub fn socket(&self) -> &mio::tcp::TcpStream {
        &self.socket
    }

    pub fn address(&self) -> &net::SocketAddr {
        &self.address
    }

    fn read(&mut self) -> io::Result<Option<usize>> {
        if self.position == HEADER_SIZE &&
           self.data.actual_length() == HEADER_SIZE {
            self.data.allocate_payload();
        }
        self.socket.try_read(&mut self.data.as_mut()[self.position..])
    }

    pub fn readable(&mut self) -> io::Result<Option<MessageData>> {
        // TODO: data length == 0?
        // TODO: maximum data length?
        loop {
            match try!(self.read()) {
                None | Some(0) => return Ok(None),
                Some(n) => {
                    self.position += n;
                    if self.position >= HEADER_SIZE &&
                       self.position == self.data.total_length() {
                        let mut data = MessageData::new();
                        mem::swap(&mut data, &mut self.data);
                        self.position = 0;
                        return Ok(Some(data))
                    }
                }
            }
        }
    }
}

impl OutgoingConnection {
    pub fn new(socket: mio::tcp::TcpStream, address: net::SocketAddr)
            -> OutgoingConnection {
        OutgoingConnection {
            socket: socket,
            address: address,
            queue: collections::VecDeque::new(),
            position: 0
        }
    }

    pub fn socket(&self) -> &mio::tcp::TcpStream {
        &self.socket
    }

    pub fn address(&self) -> &net::SocketAddr {
        &self.address
    }

    pub fn writable(&mut self) -> io::Result<()> {
        // TODO: use try_write_buf
        while let Some(message) = self.queue.pop_front() {
            match try!(self.socket.try_write(message.as_ref().as_ref())) {
                None | Some(0) => {
                    self.queue.push_front(message);
                    break
                },
                Some(n) => {
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

    pub fn send(&mut self, message: Message) {
        // TODO: capacity overflow
        // TODO: reregister
        self.queue.push_back(message);
    }

    pub fn is_idle(&self) -> bool {
        self.queue.is_empty()
    }
}
