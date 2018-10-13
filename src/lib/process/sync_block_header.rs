use std::sync::{Arc, Mutex};
use failure::Error;
use futures::future::{loop_fn, Future, Loop};
use bitcoin::blockdata::block::LoneBlockHeader;

use blockchain::BlockChain;
use connection::{Connection, ConnectionError};

const NUM_MAX_HEADERS_IN_MSG: usize = 2000;

pub fn start_sync_block_header(
    blockchain: Arc<Mutex<BlockChain>>,
    conn: Connection,
) -> impl Future<Item = Connection, Error = Error>
{
    loop_fn((blockchain, conn), |(blockchain, conn)| {
        let locator_hashes = {
            let lock = blockchain.lock().unwrap();
            lock.active_chain().locator_hashes_vec()
        };
        let blockchain2 = blockchain.clone();
        conn.getheaders(locator_hashes)
            .and_then(move |(headers, conn)| {
                let is_complete = headers.len() != NUM_MAX_HEADERS_IN_MSG;
                apply_all_headers(blockchain.clone(), headers)?;
                Ok((is_complete, conn))
            })
            .map(move |(is_complete, conn)| {
                if is_complete {
                    Loop::Break(conn)
                } else {
                    Loop::Continue((blockchain2, conn))
                }
            })
    })
}

fn apply_all_headers(blockchain: Arc<Mutex<BlockChain>>, headers: Vec<LoneBlockHeader>) -> Result<(), Error>
{
    let mut lock = blockchain.lock().unwrap();
    for header in headers {
        if let Err(_) = lock.try_add(header.header) {
            bail!(ConnectionError::MisbehavePeer);
        }
    }
    Ok(())
}
