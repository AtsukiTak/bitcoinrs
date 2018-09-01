use std::time::{SystemTime, UNIX_EPOCH};
use bitcoin::network::{constants, address::Address, message::NetworkMessage,
                       message_blockdata::{GetHeadersMessage, InvType, Inventory}, message_network::VersionMessage,
                       serialize::BitcoinHash};
use bitcoin::blockdata::block::{Block, BlockHeader, LoneBlockHeader};
use bitcoin::util::hash::Sha256dHash;
use futures::future::{loop_fn, result, Future, Loop};

use socket::AsyncSocket;
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


/*
 * High level functions
 */

pub fn getheaders(
    conn: Connection,
    locator_hashes: Vec<Sha256dHash>,
) -> impl Future<Item = (Connection, Vec<BlockHeader>), Error = Error>
{
    request_getheaders(conn, locator_hashes)
        .and_then(wait_recv_headers)
        .and_then(move |(conn, headers)| {
            if headers.is_empty() {
                info!("Peer {} sends empty headers message", conn);
                return Err(Error::from(ErrorKind::MisbehaviorPeer(conn)));
            }
            Ok((conn, headers))
        })
}

pub fn getblocks(
    conn: Connection,
    block_hashes: Vec<Sha256dHash>,
) -> impl Future<Item = (Connection, Vec<Block>), Error = Error>
{
    let n_req_blocks = block_hashes.len();
    request_getblocks(conn, block_hashes.clone())
        .and_then(move |conn| wait_recv_blocks(conn, n_req_blocks))
        .and_then(move |(conn, blocks)| {
            let is_expected_blocks = blocks
                .iter()
                .zip(block_hashes.iter())
                .all(|(block, hash)| block.bitcoin_hash() == *hash);
            if !is_expected_blocks {
                Err(Error::from(ErrorKind::MisbehaviorPeer(conn)))
            } else {
                Ok((conn, blocks))
            }
        })
}

/*
 * Low level functions
 */

pub fn request_getheaders(
    conn: Connection,
    locator_hashes: Vec<Sha256dHash>,
) -> impl Future<Item = Connection, Error = Error>
{
    let get_headers_msg = GetHeadersMessage::new(locator_hashes, Sha256dHash::default());
    let msg = OutgoingMessage::GetHeaders(get_headers_msg);
    conn.send_msg(msg)
}

pub fn wait_recv_headers(conn: Connection) -> impl Future<Item = (Connection, Vec<BlockHeader>), Error = Error>
{
    conn.recv_msg().then(|res| {
        match res? {
            (IncomingMessage::Headers(hs), conn) => {
                info!("Receive headers message");
                let headers = hs.iter().map(|lone| lone.header).collect();
                Ok((conn, headers))
            },
            (msg, conn) => {
                info!("Receive unexpected message. Expected headers msg but receive {}", msg);
                Err(Error::from(ErrorKind::MisbehaviorPeer(conn)))
            },
        }
    })
}


pub fn request_getblocks(
    conn: Connection,
    block_hashes: Vec<Sha256dHash>,
) -> impl Future<Item = Connection, Error = Error>
{
    let invs: Vec<_> = block_hashes
        .iter()
        .map(|hash| {
            Inventory {
                inv_type: InvType::Block,
                hash: *hash,
            }
        })
        .collect();
    let msg = OutgoingMessage::GetData(invs);
    conn.send_msg(msg)
}


pub fn wait_recv_blocks(
    conn: Connection,
    n_req_blocks: usize,
) -> impl Future<Item = (Connection, Vec<Block>), Error = Error>
{
    loop_fn((conn, vec![], n_req_blocks), |(conn, mut blocks_buf, n_req_blocks)| {
        conn.recv_msg().then(move |res| {
            match res? {
                // Receive "block" message
                (IncomingMessage::Block(block), conn) => {
                    info!("Receive a new block");
                    blocks_buf.push(block);
                    let n_rmn_blocks = n_req_blocks - 1;

                    if n_rmn_blocks == 0 {
                        Ok(Loop::Break((conn, blocks_buf)))
                    } else {
                        Ok(Loop::Continue((conn, blocks_buf, n_rmn_blocks)))
                    }
                },
                // Errors
                (msg, conn) => {
                    info!("Receive unexpected message. Expected block msg but receive {}", msg);
                    info!("Drop connection {:?}", conn);
                    Err(Error::from(ErrorKind::MisbehaviorPeer(conn)))
                },
            }
        })
    })
}

pub fn wait_recv_inv(conn: Connection) -> impl Future<Item = (Connection, Vec<Inventory>), Error = Error>
{
    conn.recv_msg().then(|res| {
        match res? {
            (IncomingMessage::Inv(invs), conn) => Ok((conn, invs)),
            (msg, conn) => {
                info!("Receive unexpected message. Expected headers msg but receive {}", msg);
                Err(Error::from(ErrorKind::MisbehaviorPeer(conn)))
            },
        }
    })
}
