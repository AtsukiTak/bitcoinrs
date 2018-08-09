use bitcoin::network::serialize::BitcoinHash;
use bitcoin::blockdata::block::{Block, BlockHeader};
use futures::{future::{loop_fn, Future, Loop}, stream::{iter_ok, unfold, Stream}};
use std::cmp::min;

use connection::Connection;
use blockchain::{BlockChainMut, BlockData};
use error::{Error, ErrorKind};
use super::{getblocks, getheaders};

#[derive(Debug)]
pub enum DownloadResult
{
    NewBlock(Block),
    Completed(Connection, BlockChainMut),
}

/// Initial block download process.
/// Returned stream emits `Block`s; which starts at next to `start_block` and ends latest
/// block. When process is completed, finally `Connection` is returned.
/// Note that `start_block` must be a stabled one such as genesis block or
/// enough confirmed block.
pub fn initial_block_download(
    conn: Connection,
    block_chain: BlockChainMut,
) -> impl Stream<Item = DownloadResult, Error = Error>
{
    download_all_headers(conn, block_chain) // Future<Item = (Connection, BlockChainMut, Vec<BlockHeader>)>
        .and_then(|(conn, block_chain, headers)| Ok(download_all_blocks(conn, headers, block_chain)))
        .into_stream() // Stream<Item = Stream>
        .flatten()
}

fn download_all_headers(
    conn: Connection,
    block_chain: BlockChainMut,
) -> impl Future<Item = (Connection, BlockChainMut, Vec<BlockHeader>), Error = Error>
{
    const MAX_HEADERS_IN_MSG: usize = 2000;

    loop_fn(
        (conn, block_chain, Vec::new()), // Initial state
        |(conn, mut block_chain, mut headers_buf)| {
            let locator_hashes = block_chain.locator_blocks().map(|b| b.bitcoin_hash()).collect();
            getheaders(conn, locator_hashes).and_then(|(conn, mut headers)| {
                info!("Received new {} headers", headers.len());
                let is_completed = headers.len() != MAX_HEADERS_IN_MSG;

                // Try append each header into internal blockchain.
                for h in headers.iter() {
                    match block_chain.try_add(BlockData::new(h.clone())) {
                        Ok(_) => {},
                        Err(_e) => {
                            warn!(
                                "Peer {} sends an invalid header. We can't apply it into internal blockchain.",
                                conn
                            );
                            return Err(Error::from(ErrorKind::MisbehaviorPeer(conn)));
                        },
                    }
                }

                headers_buf.append(&mut headers);

                match is_completed {
                    true => Ok(Loop::Break((conn, block_chain, headers_buf))),
                    false => Ok(Loop::Continue((conn, block_chain, headers_buf))),
                }
            })
        },
    )
}

fn download_all_blocks(
    conn: Connection,
    new_headers: Vec<BlockHeader>,
    block_chain: BlockChainMut,
) -> impl Stream<Item = DownloadResult, Error = Error>
{
    const NUM_BLOCKS_REQ_AT_ONCE: usize = 16;

    unfold(Some((conn, new_headers, block_chain)), |maybe_items| {
        match maybe_items {
            None => None,
            Some((conn, mut req_headers, block_chain)) => {
                let n_req_blocks = min(req_headers.len(), NUM_BLOCKS_REQ_AT_ONCE);
                let remain_headers = req_headers.split_off(n_req_blocks);
                let hashes = req_headers.iter().map(|h| h.bitcoin_hash()).collect();
                let download_fut = getblocks(conn, hashes) // Future<Item = (Vec<Block>, Connection)>
                    .map(move |(conn, mut blocks)| {
                        let mut download_results: Vec<_> = blocks.drain(..).map(|b| DownloadResult::NewBlock(b)).collect();
                        if remain_headers.is_empty() {
                            download_results.push(DownloadResult::Completed(conn, block_chain));
                            (iter_ok(download_results), None)
                        } else {
                            (iter_ok(download_results), Some((conn, remain_headers, block_chain)))
                        }
                    }); // Future<Item = (Stream<DownloadResult>, (Connection, Vec<BlockHeader>)>
                Some(download_fut)
            },
        }
    }) // Stream<Item = Stream<Item = DownloadResult>>
    .flatten() // Stream<Item = DownloadResult>
}
