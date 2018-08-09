use bitcoin::blockdata::block::Block;
use bitcoin::network::serialize::BitcoinHash;
use futures::{Future, stream::{unfold, Stream}};

use connection::{Connection, IncomingMessage};
use blockchain::{BlockChain, BlockChainMut, BlockData};
use error::{Error, ErrorKind};
use super::{getblocks, getheaders};

pub fn listen_new_block(
    conn: Connection,
    block_chain: BlockChainMut,
) -> impl Stream<Item = (BlockChain, Vec<Block>), Error = Error>
{
    unfold((conn, block_chain), |(conn, block_chain)| {
        let f = listen_single_process(conn, block_chain).map(|(conn, block_chain, blocks)| {
            let chain = block_chain.freeze();
            ((chain, blocks), (conn, block_chain))
        });
        Some(f)
    })
}

fn listen_single_process(
    conn: Connection,
    mut block_chain: BlockChainMut,
) -> impl Future<Item = (Connection, BlockChainMut, Vec<Block>), Error = Error>
{
    let locator_hashes = block_chain.locator_blocks().map(|b| b.bitcoin_hash()).collect();
    conn.recv_msg()
        .and_then(|(msg, conn)| {
            match msg {
                // If we use "standard block relay", peer sends "inv" message first.
                // Or even if we have signalled "sendheaders", peer still may send "inv" message first.
                IncomingMessage::Inv(_invs) => Ok(conn),

                // If we have signalled "sendheaders", we may use "direct headers announcement".
                // In that case, peer may send "headers" message instead of "inv" message.
                // For our current implementation, we don't use this feature so we just disconnect if
                // we received headers message first.
                IncomingMessage::Headers(_) => {
                    warn!("Expect inv message but receive headers message.");
                    Err(Error::from(ErrorKind::MisbehaviorPeer(conn)))
                },

                IncomingMessage::Block(_) => {
                    warn!("Expect inv message but receive block message.");
                    Err(Error::from(ErrorKind::MisbehaviorPeer(conn)))
                },
            }
        })

        // re-fetch headers newer than I have
        .and_then(move |conn| getheaders(conn, locator_hashes)) // Future<Item = (Connection, Vec<BlockHeader>)>

        // try to apply to internal blockchain
        .and_then(move |(conn, headers)| {
            for header in headers.iter() {
                match block_chain.try_add(BlockData::new(header.clone())) {
                    Ok(_) => {},
                    Err(_e) => {
                        warn!("Peer {} sends invalid header", conn);
                        return Err(Error::from(ErrorKind::MisbehaviorPeer(conn)));
                    },
                }
            }
            Ok((conn, block_chain, headers))
        })

        // getblocks
        .and_then(|(conn, block_chain, headers)| {
            let hashes = headers.iter().map(|h| h.bitcoin_hash()).collect();
            getblocks(conn, hashes).map(|(conn, blocks)| (conn, block_chain, blocks))
        })
}
