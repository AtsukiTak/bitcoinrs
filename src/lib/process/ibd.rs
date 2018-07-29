use bitcoin::network::serialize::BitcoinHash;
use bitcoin::blockdata::block::{Block, BlockHeader};
use bitcoin::util::hash::Sha256dHash;
use futures::{future::{loop_fn, Future, Loop}, stream::{iter_ok, unfold, Stream}};
use std::cmp::min;

use connection::Connection;
use super::{getblocks, getheaders, ProcessError};

pub enum DownloadResult
{
    NewBlock(Block),
    Completed(Connection),
}

/// Initial block download process.
/// Returned stream emits `Block`s; which starts at next to `start_block` and ends latest
/// block. When process is completed, finally `Connection` is returned.
/// Note that `start_block` must be a stabled one such as genesis block or
/// enough confirmed block.
pub fn initial_block_download(
    conn: Connection,
    start_block_hash: Sha256dHash,
) -> impl Stream<Item = DownloadResult, Error = ProcessError>
{
    download_all_headers(conn, start_block_hash) // Future<Item = (Connection, Vec<BlockHeader>)>
        .and_then(|(conn, headers)| Ok(download_all_blocks(conn, headers))) // Future<Item = Future<Item = Stream>>
        .into_stream() // Stream<Item = Stream>
        .flatten()
}

fn download_all_headers(
    conn: Connection,
    start_block_hash: Sha256dHash,
) -> impl Future<Item = (Connection, Vec<BlockHeader>), Error = ProcessError>
{
    const MAX_HEADERS_IN_MSG: usize = 2000;

    loop_fn(
        (conn, start_block_hash, Vec::<BlockHeader>::new()),
        |(conn, start_block_hash, mut headers_buf)| {
            getheaders(conn, start_block_hash).and_then(|(conn, mut headers)| {
                let is_completed = headers.len() == MAX_HEADERS_IN_MSG;
                headers_buf.append(&mut headers);

                if is_completed {
                    Ok(Loop::Break((conn, headers_buf)))
                } else {
                    let last_hash = headers_buf.last().unwrap().bitcoin_hash();
                    Ok(Loop::Continue((conn, last_hash, headers_buf)))
                }
            })
        },
    )
}

fn download_all_blocks(
    conn: Connection,
    all_headers: Vec<BlockHeader>,
) -> impl Stream<Item = DownloadResult, Error = ProcessError>
{
    const NUM_BLOCKS_REQ_AT_ONCE: usize = 16;

    unfold(Some((conn, all_headers)), |maybe_items| {
        match maybe_items {
            None => None,
            Some((conn, mut headers)) => {
                let n_req_blocks = min(headers.len(), NUM_BLOCKS_REQ_AT_ONCE);
                let remain_headers = headers.split_off(n_req_blocks);
                let hashes = headers.iter().map(|h| h.bitcoin_hash()).collect();
                let download_fut = getblocks(conn, hashes) // Future<Item = (Vec<Block>, Connection)>
                    .map(move |(conn, mut blocks)| {
                        let mut download_results: Vec<_> = blocks.drain(..).map(|b| DownloadResult::NewBlock(b)).collect();
                        if remain_headers.is_empty() {
                            download_results.push(DownloadResult::Completed(conn));
                            (iter_ok(download_results), None)
                        } else {
                            (iter_ok(download_results), Some((conn, remain_headers)))
                        }
                    }); // Future<Item = (Stream<DownloadResult>, (Connection, Vec<BlockHeader>)>
                Some(download_fut)
            },
        }
    }) // Stream<Item = Stream<Item = DownloadResult>>
    .flatten() // Stream<Item = DownloadResult>
}
