use std::cmp::min;
use bitcoin::blockdata::block::{Block, BlockHeader};
use bitcoin::network::{address::Address, message_blockdata::{GetHeadersMessage, InvType, Inventory},
                       serialize::BitcoinHash};
use bitcoin::util::hash::Sha256dHash;
use futures::future::{loop_fn, Future, Loop};

use error::{Error, ErrorKind};
use blockchain::{BlockChain, BlockData, FullBlockData};
use super::connection::{Connection, IncomingMessage, OutgoingMessage};

const DEFAULT_NUM_MAX_ADDRESS: usize = 8;
const DEFAULT_NUM_MAX_INVS: usize = 0;

const MAX_HEADERS_IN_MSG: usize = 2000;
const MAX_BLOCKS_IN_MSG: usize = 500;

/// The responsibilities of `Peer` is
/// - to send request and receive response
/// - to store some incoming information such as another peer address
pub struct Peer
{
    conn: Connection,
    unexpected_invs: InventoryManager,
    peer_address: PeerAddressManager,
}

struct InventoryManager
{
    n_max_invs: usize,
    invs: Vec<Inventory>,
}

struct PeerAddressManager
{
    n_max_addrs: usize,
    addrs: Vec<(u32, Address)>,
}

impl Peer
{
    pub fn new(conn: Connection) -> Peer
    {
        Peer {
            conn,
            unexpected_invs: InventoryManager::new(),
            peer_address: PeerAddressManager::new(),
        }
    }

    fn break_down(self) -> (Connection, InventoryManager, PeerAddressManager)
    {
        (self.conn, self.unexpected_invs, self.peer_address)
    }

    fn constract(conn: Connection, invs: InventoryManager, addrs: PeerAddressManager) -> Peer
    {
        Peer {
            conn,
            unexpected_invs: invs,
            peer_address: addrs,
        }
    }

    pub fn sync_blockchain(self, blockchain: BlockChain) -> impl Future<Item = (Peer, BlockChain), Error = Error>
    {
        loop_fn((self, blockchain), |(peer, mut blockchain)| {
            let locator_hashes = blockchain.active_chain().locator_hashes_vec();
            getheaders(peer, locator_hashes).and_then(move |(headers, peer)| {
                info!("Received new {} headers", headers.len());

                let is_completed = headers.len() != MAX_HEADERS_IN_MSG;

                for header in headers {
                    if let Err(_) = blockchain.try_add(header) {
                        return Err(Error::from(ErrorKind::MisbehaviorPeer(peer.conn)));
                    }
                }

                info!(
                    "Current height is {}",
                    blockchain.active_chain().latest_block().height()
                );

                match is_completed {
                    true => Ok(Loop::Break((peer, blockchain))),
                    false => Ok(Loop::Continue((peer, blockchain))),
                }
            })
        })
    }

    pub fn download_full_blocks(
        self,
        req_blocks: Vec<BlockData>,
    ) -> impl Future<Item = (Peer, Vec<FullBlockData>), Error = Error>
    {
        let full_blocks_buf = Vec::with_capacity(req_blocks.len());
        loop_fn(
            (self, req_blocks, full_blocks_buf),
            |(peer, mut req_blocks, mut full_blocks_buf)| {
                let n_req_blocks = min(req_blocks.len(), MAX_BLOCKS_IN_MSG);
                let rmn_blocks = req_blocks.split_off(n_req_blocks);
                let req_blocks_hashes = req_blocks.iter().map(|b| b.bitcoin_hash()).collect();

                getblocks(peer, req_blocks_hashes).map(move |(peer, full_blocks)| {
                    info!("Downloaded {} full blocks", full_blocks.len());

                    let full_block_datas = full_blocks
                        .into_iter()
                        .zip(req_blocks)
                        .map(|(block, data)| FullBlockData::new(block, data.height()));

                    for full_block_data in full_block_datas {
                        full_blocks_buf.push(full_block_data);
                    }

                    match rmn_blocks.is_empty() {
                        true => Loop::Break((peer, full_blocks_buf)),
                        false => Loop::Continue((peer, rmn_blocks, full_blocks_buf)),
                    }
                })
            },
        )
    }
}

impl InventoryManager
{
    fn new() -> InventoryManager
    {
        InventoryManager {
            n_max_invs: DEFAULT_NUM_MAX_INVS,
            invs: Vec::new(),
        }
    }

    fn set_n_max_invs(&mut self, n_max_invs: usize)
    {
        self.n_max_invs = n_max_invs;
    }

    fn append(&mut self, mut invs: Vec<Inventory>)
    {
        self.invs.append(&mut invs);
    }
}

impl PeerAddressManager
{
    fn new() -> PeerAddressManager
    {
        PeerAddressManager {
            n_max_addrs: DEFAULT_NUM_MAX_ADDRESS,
            addrs: Vec::new(),
        }
    }

    fn set_n_max_address(&mut self, n_max_addrs: usize)
    {
        self.n_max_addrs = n_max_addrs;
    }

    fn append(&mut self, mut addrs: Vec<(u32, Address)>)
    {
        self.addrs.append(&mut addrs);
        // TODO remove exceed items
    }
}

/* Internal functions */

fn getheaders(
    peer: Peer,
    locator_hashes: Vec<Sha256dHash>,
) -> impl Future<Item = (Vec<BlockHeader>, Peer), Error = Error>
{
    request_getheaders(peer, locator_hashes)
        .and_then(recv_headers)
        .and_then(move |(headers, peer)| {
            if headers.is_empty() {
                warn!("Peer {} sends empty headers message", peer.conn);
                return Err(Error::from(ErrorKind::MisbehaviorPeer(peer.conn)));
            }
            Ok((headers, peer))
        })
}

fn request_getheaders(peer: Peer, locator_hashes: Vec<Sha256dHash>) -> impl Future<Item = Peer, Error = Error>
{
    let (conn, invs, addrs) = peer.break_down();
    let get_headers_msg = GetHeadersMessage::new(locator_hashes, Sha256dHash::default());
    let msg = OutgoingMessage::GetHeaders(get_headers_msg);
    conn.send_msg(msg).map(move |conn| {
        Peer {
            conn,
            unexpected_invs: invs,
            peer_address: addrs,
        }
    })
}

fn recv_headers(peer: Peer) -> impl Future<Item = (Vec<BlockHeader>, Peer), Error = Error>
{
    loop_fn(peer, |peer| {
        let (conn, mut inner_invs, mut inner_addrs) = peer.break_down();
        conn.recv_msg().and_then(move |(msg, conn)| {
            match msg {
                IncomingMessage::Headers(hs) => {
                    let peer = Peer::constract(conn, inner_invs, inner_addrs);
                    let hs = hs.into_iter().map(|lone| lone.header).collect();
                    Ok(Loop::Break((hs, peer)))
                },
                IncomingMessage::Inv(invs) => {
                    inner_invs.append(invs);
                    let peer = Peer::constract(conn, inner_invs, inner_addrs);
                    Ok(Loop::Continue(peer))
                },
                IncomingMessage::Addr(addrs) => {
                    inner_addrs.append(addrs);
                    let peer = Peer::constract(conn, inner_invs, inner_addrs);
                    Ok(Loop::Continue(peer))
                },
                IncomingMessage::Block(_) => Err(Error::from(ErrorKind::MisbehaviorPeer(conn))),
            }
        })
    })
}

fn getblocks(peer: Peer, block_hashes: Vec<Sha256dHash>) -> impl Future<Item = (Peer, Vec<Block>), Error = Error>
{
    let n_req_blocks = block_hashes.len();
    request_getblocks(peer, block_hashes.clone())
        .and_then(move |peer| recv_blocks(peer, n_req_blocks))
        .and_then(move |(peer, blocks)| {
            let is_expected_blocks = blocks
                .iter()
                .zip(block_hashes.iter())
                .all(|(block, hash)| block.bitcoin_hash() == *hash);
            if !is_expected_blocks {
                Err(Error::from(ErrorKind::MisbehaviorPeer(peer.conn)))
            } else {
                Ok((peer, blocks))
            }
        })
}

fn request_getblocks(peer: Peer, block_hashes: Vec<Sha256dHash>) -> impl Future<Item = Peer, Error = Error>
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
    let (conn, inner_invs, inner_addrs) = peer.break_down();
    conn.send_msg(msg)
        .map(move |conn| Peer::constract(conn, inner_invs, inner_addrs))
}

fn recv_blocks(peer: Peer, n_req_blocks: usize) -> impl Future<Item = (Peer, Vec<Block>), Error = Error>
{
    let blocks_buf = Vec::with_capacity(n_req_blocks);
    loop_fn(
        (peer, blocks_buf, n_req_blocks), // Initial args
        |(peer, mut blocks_buf, n_req_blocks)| {
            let (conn, mut inner_invs, mut inner_addrs) = peer.break_down();
            conn.recv_msg().and_then(move |(msg, conn)| {
                match msg {
                    IncomingMessage::Block(b) => {
                        info!("Receve a new block");
                        blocks_buf.push(b);
                        let n_rmn_blocks = n_req_blocks - 1;

                        let peer = Peer::constract(conn, inner_invs, inner_addrs);
                        if n_rmn_blocks == 0 {
                            Ok(Loop::Break((peer, blocks_buf)))
                        } else {
                            Ok(Loop::Continue((peer, blocks_buf, n_rmn_blocks)))
                        }
                    },
                    IncomingMessage::Inv(invs) => {
                        inner_invs.append(invs);
                        let peer = Peer::constract(conn, inner_invs, inner_addrs);
                        Ok(Loop::Continue((peer, blocks_buf, n_req_blocks)))
                    },
                    IncomingMessage::Addr(addrs) => {
                        inner_addrs.append(addrs);
                        let peer = Peer::constract(conn, inner_invs, inner_addrs);
                        Ok(Loop::Continue((peer, blocks_buf, n_req_blocks)))
                    },
                    IncomingMessage::Headers(_) => Err(Error::from(ErrorKind::MisbehaviorPeer(conn))),
                }
            })
        },
    )
}
