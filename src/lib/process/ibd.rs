use bitcoin::network::serialize::BitcoinHash;
use bitcoin::blockdata::block::BlockHeader;
use bitcoin::util::hash::Sha256dHash;
use futures::future::{loop_fn, Future, Loop};
use std::cmp::min;

use connection::Connection;
use blockchain::BlockChainMut;
use error::{Error, ErrorKind};
use super::{getblocks, getheaders};

/// Initial block download process.
/// Returned stream emits `Block`s; which starts at next to `start_block` and ends latest
/// block. When process is completed, finally `Connection` is returned.
/// Note that `start_block` must be a stabled one such as genesis block or
/// enough confirmed block.
pub fn initial_block_download(
    conn: Connection,
    block_chain: BlockChainMut,
) -> impl Future<Item = (Connection, BlockChainMut), Error = Error>
{
    let locator_hashes: Vec<Sha256dHash> = {
        let mut vec = Vec::new();
        let active_chain = block_chain.active_chain();
        for hash in active_chain.locator_hashes() {
            vec.push(hash);
        }
        vec
    };
    download_all_headers(conn, locator_hashes)
        .and_then(move |(conn, headers)| download_all_blocks(conn, headers, block_chain))
}

fn download_all_headers(
    conn: Connection,
    locator_hashes: Vec<Sha256dHash>,
) -> impl Future<Item = (Connection, Vec<BlockHeader>), Error = Error>
{
    const MAX_HEADERS_IN_MSG: usize = 2000;

    loop_fn(
        (conn, locator_hashes, Vec::new()), // Initial state
        |(conn, locator_hashes, mut headers_buf)| {
            getheaders(conn, locator_hashes).and_then(|(conn, mut headers)| {
                info!("Received new {} headers", headers.len());
                let is_completed = headers.len() != MAX_HEADERS_IN_MSG;

                headers_buf.append(&mut headers);
                let next_locator_hashes = vec![headers_buf.last().unwrap().bitcoin_hash()];

                match is_completed {
                    true => Ok(Loop::Break((conn, headers_buf))),
                    false => Ok(Loop::Continue((conn, next_locator_hashes, headers_buf))),
                }
            })
        },
    )
}

fn download_all_blocks(
    conn: Connection,
    new_headers: Vec<BlockHeader>,
    block_chain: BlockChainMut,
) -> impl Future<Item = (Connection, BlockChainMut), Error = Error>
{
    const NUM_BLOCKS_REQ_AT_ONCE: usize = 16;

    loop_fn(
        (conn, new_headers, block_chain),
        |(conn, mut rmn_headers, mut block_chain)| {
            let n_req_blocks = min(rmn_headers.len(), NUM_BLOCKS_REQ_AT_ONCE);
            let req_header_hashes = rmn_headers.drain(..n_req_blocks).map(|h| h.bitcoin_hash()).collect();
            getblocks(conn, req_header_hashes).and_then(move |(conn, blocks)| {
                // Store all blocks into blockchain
                for block in blocks {
                    match block_chain.try_add(block.header) {
                        Ok(_) => info!("Added a new block"),
                        Err(_e) => {
                            warn!("Peer {} sends us an invalid block", conn);
                            return Err(Error::from(ErrorKind::MisbehaviorPeer(conn)));
                        },
                    };
                }

                match rmn_headers.is_empty() {
                    true => Ok(Loop::Break((conn, block_chain))),
                    false => Ok(Loop::Continue((conn, rmn_headers, block_chain))),
                }
            })
        },
    )
}
