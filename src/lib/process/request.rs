use bitcoin::network::{message_blockdata::{GetHeadersMessage, InvType, Inventory}, serialize::BitcoinHash};
use bitcoin::blockdata::block::{Block, BlockHeader};
use bitcoin::util::hash::Sha256dHash;
use futures::future::{loop_fn, Future, Loop};

use connection::{Connection, IncomingMessage, OutgoingMessage};
use error::{Error, ErrorKind};

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
    conn.recv_msg().then(move |res| {
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
