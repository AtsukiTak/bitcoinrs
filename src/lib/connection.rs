use std::time::{SystemTime, UNIX_EPOCH};
use bitcoin::network::{constants, message::NetworkMessage, message_network::VersionMessage};

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
                error!("Expect Version msg but found {:?}", msg);
                return Err(Error::from(ErrorKind::InvalidPeer));
            },
        };

        // Send Verack msg
        socket.send_msg(NetworkMessage::Verack)?;

        // Receive Verack msg
        match socket.recv_msg()? {
            NetworkMessage::Verack => {},
            msg => {
                error!("Expect Verack msg but found {:?}", msg);
                return Err(Error::from(ErrorKind::InvalidPeer));
            },
        }

        Ok(Connection {
            socket,
            remote_version_msg,
            local_version_msg,
        })
    }

    pub fn send_msg(&mut self, msg: NetworkMessage) -> Result<(), Error>
    {
        self.socket.send_msg(msg)
    }

    /// Receive only
    /// - Block
    /// - Inv
    /// message.
    ///
    /// Wait until above message comes.
    pub fn recv_msg(&mut self) -> Result<NetworkMessage, Error>
    {
        loop {
            let msg = self.socket.recv_msg()?;
            info!("Receive a new message : {:?}", msg);
            match msg {
                NetworkMessage::Ping(nonce) => self.send_msg(NetworkMessage::Pong(nonce))?,
                m @ NetworkMessage::Block(_) => return Ok(m),
                m @ NetworkMessage::Inv(_) => return Ok(m),
                _ => {
                    info!("Discard incoming message.");
                },
            }
        }
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
