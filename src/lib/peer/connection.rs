use std::time::{SystemTime, UNIX_EPOCH};
use bitcoin::network::{constants, address::Address, message::NetworkMessage,
                       message_blockdata::{GetHeadersMessage, InvType, Inventory}, message_network::VersionMessage,
                       serialize::BitcoinHash};
use bitcoin::blockdata::block::{Block, LoneBlockHeader};
use bitcoin::util::hash::Sha256dHash;
use futures::future::{loop_fn, result, Future, Loop};

use peer::socket::AsyncSocket;
use error::{Error, ErrorKind};

/// Connection between two peers.
/// The responsibilities of this layer is
/// - complete handshake
/// - restrict incoming/outgoing message types
#[derive(Debug)]
pub struct Connection
{
    socket: AsyncSocket,

    remote_version_msg: VersionMessage,
    local_version_msg: VersionMessage,
}

pub enum OutgoingMessage
{
    GetHeaders(GetHeadersMessage),
    GetData(Vec<Inventory>),
}

pub enum IncomingMessage
{
    Headers(Vec<LoneBlockHeader>),
    Block(Block),
    Inv(Vec<Inventory>),
    Addr(Vec<(u32, Address)>),
}

impl Connection
{
    pub fn initialize(socket: AsyncSocket, start_height: i32) -> impl Future<Item = Connection, Error = Error>
    {
        // Send Version msg
        let local_version_msg = version_msg(&socket, start_height);
        socket
            .send_msg(NetworkMessage::Version(local_version_msg.clone()))
            .and_then(|socket| socket.recv_msg())
            .and_then(|(msg, socket)| {
                // Receive Version msg
                match msg {
                    NetworkMessage::Version(v) => Ok((v, socket)),
                    msg => {
                        warn!("Expect Version msg but found {:?}", msg);
                        Err(Error::from(ErrorKind::HandshakeError(socket)))
                    },
                }
            })
            .and_then(|(remote_v, socket)| socket.send_msg(NetworkMessage::Verack).map(|s| (s, remote_v)))
            .and_then(|(socket, remote_v)| socket.recv_msg().map(|(msg, s)| (msg, s, remote_v)))
            .and_then(move |(msg, socket, remote_v)| {
                // Receive Verack msg
                match msg {
                    NetworkMessage::Verack => {
                        Ok(Connection {
                            socket,
                            remote_version_msg: remote_v,
                            local_version_msg,
                        })
                    },
                    msg => {
                        warn!("Expect Verack msg but found {:?}", msg);
                        Err(Error::from(ErrorKind::HandshakeError(socket)))
                    },
                }
            })
    }

    /// Send only below message.
    /// - GetBlocks
    /// - GetData
    pub fn send_msg(self, msg: OutgoingMessage) -> impl Future<Item = Self, Error = Error>
    {
        let (socket, remote_v, local_v) = (self.socket, self.remote_version_msg, self.local_version_msg);
        info!("Send {}", msg);
        let msg = match msg {
            OutgoingMessage::GetHeaders(m) => NetworkMessage::GetHeaders(m),
            OutgoingMessage::GetData(m) => NetworkMessage::GetData(m),
        };
        socket.send_msg(msg).map(|socket| {
            Connection {
                socket,
                remote_version_msg: remote_v,
                local_version_msg: local_v,
            }
        })
    }

    /// Receive only below message.
    /// - Headers
    /// - Block
    /// - Inv
    pub fn recv_msg(self) -> impl Future<Item = (IncomingMessage, Self), Error = Error>
    {
        let (socket, remote_v, local_v) = (self.socket, self.remote_version_msg, self.local_version_msg);

        loop_fn(socket, |socket| {
            socket
                .recv_msg()
                .map_err(|e| Err(e)) // Future<Item = _, Error = Result<Error>>
                .and_then(|(msg, socket)| {
                    match msg {
                        NetworkMessage::Ping(nonce) => Err(Ok((nonce, socket))),
                        NetworkMessage::Headers(h) => Ok(Loop::Break((IncomingMessage::Headers(h), socket))),
                        NetworkMessage::Block(b) => Ok(Loop::Break((IncomingMessage::Block(b), socket))),
                        NetworkMessage::Inv(i) => Ok(Loop::Break((IncomingMessage::Inv(i), socket))),
                        NetworkMessage::Addr(a) => Ok(Loop::Break((IncomingMessage::Addr(a), socket))),
                        m => {
                            info!("Discard incoming message.");
                            debug!("Message : {:?}", m);
                            Ok(Loop::Continue(socket))
                        },
                    }
                })
                .or_else(|e_or_nonce| {
                    result(e_or_nonce).and_then(|(nonce, socket)| {
                        socket
                            .send_msg(NetworkMessage::Pong(nonce))
                            .map(|socket| Loop::Continue(socket))
                    })
                })
        }).map(|(msg, socket)| {
            info!("Receive a new message {}", msg);

            let conn = Connection {
                socket,
                remote_version_msg: remote_v,
                local_version_msg: local_v,
            };

            (msg, conn)
        })
    }
}

fn version_msg(socket: &AsyncSocket, start_height: i32) -> VersionMessage
{
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
    VersionMessage {
        version: constants::PROTOCOL_VERSION,
        services: constants::SERVICES,
        timestamp,
        receiver: socket.remote_addr().clone(),
        sender: socket.local_addr().clone(),
        nonce: 0,
        user_agent: socket.user_agent().into(),
        start_height,
        relay: false,
    }
}

impl ::std::fmt::Display for Connection
{
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error>
    {
        write!(f, "Connection on socket {}", self.socket)
    }
}

impl ::std::fmt::Display for IncomingMessage
{
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error>
    {
        match self {
            IncomingMessage::Block(_) => write!(f, "Block msg"),
            IncomingMessage::Headers(_) => write!(f, "Headers msg"),
            IncomingMessage::Inv(_) => write!(f, "Inv msg"),
            IncomingMessage::Addr(_) => write!(f, "Addr msg"),
        }
    }
}

impl ::std::fmt::Display for OutgoingMessage
{
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error>
    {
        match self {
            OutgoingMessage::GetHeaders(_) => write!(f, "GetHeaders msg"),
            OutgoingMessage::GetData(_) => write!(f, "GetData msg"),
        }
    }
}
