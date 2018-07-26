use bitcoin::network::{message_blockdata::{GetHeadersMessage, InvType, Inventory}, serialize::BitcoinHash};
use bitcoin::blockdata::block::BlockHeader;
use bitcoin::util::hash::Sha256dHash;
use futures::future::{loop_fn, Future, Loop};

use connection::{Connection, IncomingMessage, OutgoingMessage};
use blockchain::{BlockChainMut, BlockData};

pub fn initial_block_download(
    conn: Connection,
    blockchain: BlockChainMut,
) -> impl Future<Item = (Connection, BlockChainMut), Error = BlockChainMut>
{
    download_headers(conn, blockchain).and_then(|(conn, blockchain)| download_blocks(conn, blockchain))
}

fn download_headers(
    conn: Connection,
    blockchain: BlockChainMut,
) -> impl Future<Item = (Connection, BlockChainMut), Error = BlockChainMut>
{
    const MAX_HEADERS_IN_MSG: usize = 2000;

    loop_fn((conn, blockchain), |(conn, blockchain)| {
        request_getheaders(conn, blockchain)
            .and_then(|(conn, blockchain)| wait_recv_headers(conn, blockchain))
            .and_then(|(headers, conn, mut blockchain)| {
                let n_headers = headers.len();
                match apply_received_headers(&mut blockchain, headers) {
                    Ok(()) => {
                        if n_headers == MAX_HEADERS_IN_MSG {
                            Ok(Loop::Continue((conn, blockchain)))
                        } else {
                            Ok(Loop::Break((conn, blockchain)))
                        }
                    },
                    Err(()) => Err(blockchain),
                }
            })
    })
}

fn request_getheaders(
    conn: Connection,
    blockchain: BlockChainMut,
) -> impl Future<Item = (Connection, BlockChainMut), Error = BlockChainMut>
{
    let locator_hashes = blockchain.locator_blocks().map(|b| b.bitcoin_hash()).collect();
    let get_headers_msg = GetHeadersMessage::new(locator_hashes, Sha256dHash::default());
    let msg = OutgoingMessage::GetHeaders(get_headers_msg);
    conn.send_msg(msg).then(move |res| {
        match res {
            Ok(conn) => {
                info!("Sent getheaders message");
                Ok((conn, blockchain))
            },
            Err(e) => {
                info!("Error while sending getheaders message : {:?}", e);
                Err(blockchain)
            },
        }
    })
}

fn wait_recv_headers(
    conn: Connection,
    blockchain: BlockChainMut,
) -> impl Future<Item = (Vec<BlockHeader>, Connection, BlockChainMut), Error = BlockChainMut>
{
    conn.recv_msg().then(move |res| {
        match res {
            Ok((IncomingMessage::Headers(hs), conn)) => {
                info!("Receive headers message");
                let headers = hs.iter().map(|lone| lone.header).collect();
                Ok((headers, conn, blockchain))
            },
            Ok((msg, conn)) => {
                info!("Receive unexpected message. Expected headers msg but receive {}", msg);
                info!("Drop connection : {:?}", conn);
                Err(blockchain)
            },
            Err(e) => {
                info!("Error while receiving headers message : {:?}", e);
                Err(blockchain)
            },
        }
    })
}

fn apply_received_headers(blockchain: &mut BlockChainMut, mut headers: Vec<BlockHeader>) -> Result<(), ()>
{
    for header in headers.drain(..) {
        if let Err(e) = blockchain.try_add_header(header) {
            info!("Receive invalid block header. {:?}", e);
            return Err(());
        }
    }
    info!(
        "Applied new headers to internal blockchain. Current length is {}",
        blockchain.len()
    );
    Ok(())
}

fn download_blocks(
    conn: Connection,
    blockchain: BlockChainMut,
) -> impl Future<Item = (Connection, BlockChainMut), Error = BlockChainMut>
{
    loop_fn((conn, blockchain), |(conn, blockchain)| {
        request_getblocks(conn, blockchain).and_then(|(n_req_blocks, conn, blockchain)| {
            wait_recv_blocks_and_apply(conn, blockchain, n_req_blocks).map(move |(conn, blockchain)| {
                if n_req_blocks == 16 {
                    Loop::Continue((conn, blockchain))
                } else {
                    Loop::Break((conn, blockchain))
                }
            })
        })
    })
}

fn request_getblocks(
    conn: Connection,
    blockchain: BlockChainMut,
) -> impl Future<Item = (usize, Connection, BlockChainMut), Error = BlockChainMut>
{
    const DL_AT_ONCE_MAX: usize = 16;
    let invs: Vec<_> = blockchain
            .iter()
            .rev() // Make it easy to find header_only block
            .filter(|block| block.is_header_only())
            .take(DL_AT_ONCE_MAX)
            .map(|block| {
                Inventory {
                    inv_type: InvType::Block,
                    hash: block.bitcoin_hash(),
                }
            })
            .collect();
    let n_invs = invs.len();
    let msg = OutgoingMessage::GetData(invs);
    conn.send_msg(msg).then(move |res| {
        match res {
            Ok(conn) => {
                info!("Sent getblocks message");
                Ok((n_invs, conn, blockchain))
            },
            Err(e) => {
                info!("Error while sending getblocks message : {:?}", e);
                Err(blockchain)
            },
        }
    })
}

fn wait_recv_blocks_and_apply(
    conn: Connection,
    blockchain: BlockChainMut,
    n_req_blocks: usize,
) -> impl Future<Item = (Connection, BlockChainMut), Error = BlockChainMut>
{
    loop_fn(
        (conn, blockchain, n_req_blocks),
        |(conn, mut blockchain, n_req_blocks)| {
            conn.recv_msg().then(move |res| {
                match res {
                    // Receive "block" message
                    Ok((IncomingMessage::Block(block), conn)) => {
                        info!("Receive a new block");

                        // Search internal blockchain
                        let is_found = {
                            match blockchain.get_block_mut(block.bitcoin_hash()) {
                                Some(b) => {
                                    // Replace old block (it might be header only) with a new one.
                                    *b = BlockData::new_full_block(block);
                                    true
                                },
                                None => false,
                            }
                        };
                        if is_found {
                            if n_req_blocks == 1 {
                                Ok(Loop::Break((conn, blockchain)))
                            } else {
                                Ok(Loop::Continue((conn, blockchain, n_req_blocks - 1)))
                            }
                        } else {
                            info!("Receive unexpected block : Hash does not match.");
                            return Err(blockchain);
                        }
                    },

                    Ok((msg, conn)) => {
                        info!("Receive unexpected message. Expected block msg but receive {}", msg);
                        info!("Drop connection {:?}", conn);
                        Err(blockchain)
                    },
                    Err(e) => {
                        info!("Error while receiving block message : {:?}", e);
                        Err(blockchain)
                    },
                }
            })
        },
    )
}
