use std::time::{SystemTime, UNIX_EPOCH};
use bitcoin::network::{constants, message::NetworkMessage, message_blockdata::{GetHeadersMessage, Inventory},
                       message_network::VersionMessage};
use bitcoin::blockdata::block::{Block, LoneBlockHeader};

use socket::SyncSocket;
use error::{Error, ErrorKind};

/// Connection between two peers.
/// Connection handshake is handled in this layer.
#[derive(Debug)]
pub struct Connection
{
    socket: SyncSocket,

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
}

impl Connection
{
    pub fn initialize(mut socket: SyncSocket, start_height: i32) -> Result<Connection, Error>
    {
        // Send Version msg
        let local_version_msg = version_msg(&socket, start_height);
        let socket = socket.send_msg(NetworkMessage::Version(local_version_msg.clone()))?;

        // Receive Version msg
        let (recv_msg, socket) = socket.recv_msg()?;
        let remote_version_msg = match recv_msg {
            NetworkMessage::Version(v) => v,
            msg => {
                warn!("Expect Version msg but found {:?}", msg);
                return Err(Error::from(ErrorKind::InvalidPeer));
            },
        };

        // Send Verack msg
        let socket = socket.send_msg(NetworkMessage::Verack)?;

        // Receive Verack msg
        let (recv_msg, socket) = socket.recv_msg()?;
        match recv_msg {
            NetworkMessage::Verack => {},
            msg => {
                warn!("Expect Verack msg but found {:?}", msg);
                return Err(Error::from(ErrorKind::InvalidPeer));
            },
        }

        Ok(Connection {
            socket,
            remote_version_msg,
            local_version_msg,
        })
    }

    /// Send only below message.
    /// - GetBlocks
    /// - GetData
    pub fn send_msg(mut self, msg: OutgoingMessage) -> Result<Self, Error>
    {
        let (socket, remote_v, local_v) = (self.socket, self.remote_version_msg, self.local_version_msg);
        info!("Send {}", msg);
        let msg = match msg {
            OutgoingMessage::GetHeaders(m) => NetworkMessage::GetHeaders(m),
            OutgoingMessage::GetData(m) => NetworkMessage::GetData(m),
        };
        let socket = socket.send_msg(msg)?;
        Ok(Connection {
            socket,
            remote_version_msg: remote_v,
            local_version_msg: local_v,
        })
    }

    /// Receive only below message.
    /// - Block
    /// - Inv
    ///
    /// Wait until above message arrives.
    pub fn recv_msg(mut self) -> Result<(IncomingMessage, Self), Error>
    {
        fn recv_msg_inner(socket: SyncSocket) -> Result<(IncomingMessage, SyncSocket), Error>
        {
            let (msg, socket) = socket.recv_msg()?;
            match msg {
                NetworkMessage::Ping(nonce) => {
                    let socket = socket.send_msg(NetworkMessage::Pong(nonce))?;
                    recv_msg_inner(socket)
                },
                NetworkMessage::Headers(h) => Ok((IncomingMessage::Headers(h), socket)),
                NetworkMessage::Block(b) => Ok((IncomingMessage::Block(b), socket)),
                NetworkMessage::Inv(i) => Ok((IncomingMessage::Inv(i), socket)),
                _ => {
                    info!("Discard incoming message.");
                    recv_msg_inner(socket)
                },
            }
        }

        let (socket, remote_v, local_v) = (self.socket, self.remote_version_msg, self.local_version_msg);
        let (msg, socket) = recv_msg_inner(socket)?;

        info!("Receive a new message {}", msg);

        let conn = Connection {
            socket,
            remote_version_msg: remote_v,
            local_version_msg: local_v,
        };

        Ok((msg, conn))
    }
}

fn version_msg(socket: &SyncSocket, start_height: i32) -> VersionMessage
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
