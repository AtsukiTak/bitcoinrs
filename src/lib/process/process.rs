use std::cmp::min;

use futures::future::{loop_fn, Future, Loop};
use bitcoin::network::serialize::BitcoinHash;

use connection::Connection;
use blockchain::{BlockChain, BlockData, FullBlockData};
use error::{Error, ErrorKind};
use process::request::{getblocks, getheaders};

const MAX_HEADERS_IN_MSG: usize = 2000;
const MAX_BLOCKS_IN_MSG: usize = 1000;

/// Sync given `BlockChain` with latest blockchain.
/// This process only syncs `BlockHeader`.
/// If you want `Block` as well, please use `process::getblocks` function.
pub fn sync_blockchain(
    conn: Connection,
    block_chain: BlockChain,
) -> impl Future<Item = (Connection, BlockChain), Error = Error>
{
    loop_fn(
        (conn, block_chain), // Initial state
        |(conn, mut block_chain)| {
            let locator_hashes = block_chain.active_chain().locator_hashes_vec();
            getheaders(conn, locator_hashes).and_then(move |(conn, headers)| {
                info!("Received new {} headers", headers.len());

                let is_completed = headers.len() != MAX_HEADERS_IN_MSG;

                for header in headers {
                    if let Err(_) = block_chain.try_add(header) {
                        return Err(Error::from(ErrorKind::MisbehaviorPeer(conn)));
                    }
                }

                info!(
                    "Current height is {}",
                    block_chain.active_chain().latest_block().height()
                );

                match is_completed {
                    true => Ok(Loop::Break((conn, block_chain))),
                    false => Ok(Loop::Continue((conn, block_chain))),
                }
            })
        },
    )
}

/// The number of blocks can be more than MAX_BLOCKS_IN_MSG.
pub fn download_full_blocks(
    conn: Connection,
    req_blocks: Vec<BlockData>,
) -> impl Future<Item = (Connection, Vec<FullBlockData>), Error = Error>
{
    let full_blocks_buf = Vec::with_capacity(req_blocks.len());

    loop_fn(
        (conn, req_blocks, full_blocks_buf), // Initial state
        |(conn, mut req_blocks, mut full_blocks_buf)| {
            let n_req_block = min(req_blocks.len(), MAX_BLOCKS_IN_MSG);
            let rmn_blocks = req_blocks.split_off(n_req_block);
            let req_block_hashes = req_blocks.iter().map(|b| b.bitcoin_hash()).collect();
            getblocks(conn, req_block_hashes).map(move |(conn, full_blocks)| {
                info!("Downloaded {} full blocks", full_blocks.len());

                let full_block_datas = full_blocks
                    .into_iter()
                    .zip(req_blocks)
                    .map(|(block, data)| FullBlockData::new(block, data.height()));

                for full_block_data in full_block_datas {
                    full_blocks_buf.push(full_block_data);
                }

                let is_completed = rmn_blocks.is_empty();
                match is_completed {
                    true => Loop::Break((conn, full_blocks_buf)),
                    false => Loop::Continue((conn, rmn_blocks, full_blocks_buf)),
                }
            })
        },
    )
}
