use std::time::{SystemTime, UNIX_EPOCH};
use bitcoin::network::{constants, message::NetworkMessage, message_blockdata::{GetBlocksMessage, Inventory},
                       message_network::VersionMessage};
use bitcoin::blockdata::block::Block;

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
    GetBlocks(GetBlocksMessage),
    GetData(Vec<Inventory>),
}

pub enum IncomingMessage
{
    Inv(Vec<Inventory>),
    Block(Block),
}

impl Connection
{
    pub fn initialize(mut socket: SyncSocket, start_height: i32) -> Result<Connection, Error>
    {
        // Send Version msg
        let local_version_msg = version_msg(&socket, start_height);
        socket.send_msg(NetworkMessage::Version(local_version_msg.clone()))?;

        // Receive Version msg
        let remote_version_msg = match socket.recv_msg()? {
            NetworkMessage::Version(v) => v,
            msg => {
                warn!("Expect Version msg but found {:?}", msg);
                return Err(Error::from(ErrorKind::InvalidPeer));
            },
        };

        // Send Verack msg
        socket.send_msg(NetworkMessage::Verack)?;

        // Receive Verack msg
        match socket.recv_msg()? {
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
    pub fn send_msg(&mut self, msg: OutgoingMessage) -> Result<(), Error>
    {
        info!("Send {} to {}", msg, self.socket);
        let msg = match msg {
            OutgoingMessage::GetBlocks(m) => NetworkMessage::GetBlocks(m),
            OutgoingMessage::GetData(m) => NetworkMessage::GetData(m),
        };
        self.socket.send_msg(msg)
    }

    /// Receive only below message.
    /// - Block
    /// - Inv
    ///
    /// Wait until above message arrives.
    pub fn recv_msg(&mut self) -> Result<IncomingMessage, Error>
    {
        let incoming_msg = loop {
            let msg = self.socket.recv_msg()?;
            match msg {
                NetworkMessage::Ping(nonce) => self.socket.send_msg(NetworkMessage::Pong(nonce))?,
                NetworkMessage::Block(b) => break IncomingMessage::Block(b),
                NetworkMessage::Inv(i) => break IncomingMessage::Inv(i),
                _ => {
                    info!("Discard incoming message.");
                },
            }
        };

        info!("Receive a new message {} from {}", incoming_msg, self.socket);

        Ok(incoming_msg)
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

impl ::std::fmt::Display for Connection {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        write!(f, "Connection on socket {}", self.socket)
    }
}

impl ::std::fmt::Display for IncomingMessage {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        match self {
            IncomingMessage::Block(_) => write!(f, "Block msg"),
            IncomingMessage::Inv(_) => write!(f, "Inv msg"),
        }
    }
}

impl ::std::fmt::Display for OutgoingMessage {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        match self {
            OutgoingMessage::GetBlocks(_) => write!(f, "GetBlocks msg"),
            OutgoingMessage::GetData(_) => write!(f, "GetData msg"),
        }
    }
}

