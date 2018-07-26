use bitcoin::network::{message_blockdata::{GetHeadersMessage, InvType, Inventory}, serialize::BitcoinHash};
use bitcoin::blockdata::block::BlockHeader;
use bitcoin::util::hash::Sha256dHash;

use connection::{Connection, IncomingMessage, OutgoingMessage};
use blockchain::{BlockChainMut, BlockData};

pub fn initial_block_download(conn: Connection, blockchain: &mut BlockChainMut) -> Result<Connection, ()>
{
    let conn = download_headers(conn, blockchain)?;
    download_blocks(conn, blockchain)
}

fn download_headers(mut conn: Connection, blockchain: &mut BlockChainMut) -> Result<Connection, ()>
{
    const MAX_HEADERS_IN_MSG: usize = 2000;
    let conn = request_getheaders(conn, blockchain)?;
    let (headers, conn) = wait_recv_headers(conn)?;
    let n_headers = headers.len();
    apply_received_headers(blockchain, headers)?;
    if n_headers == MAX_HEADERS_IN_MSG {
        download_headers(conn, blockchain)
    } else {
        Ok(conn)
    }
}

fn request_getheaders(mut conn: Connection, blockchain: &mut BlockChainMut) -> Result<Connection, ()>
{
    let locator_hashes = blockchain.locator_blocks().map(|b| b.bitcoin_hash()).collect();
    let get_headers_msg = GetHeadersMessage::new(locator_hashes, Sha256dHash::default());
    let msg = OutgoingMessage::GetHeaders(get_headers_msg);
    match conn.send_msg(msg) {
        Ok(conn) => {
            info!("Sent getheaders message");
            Ok(conn)
        },
        Err(e) => {
            info!("Error while sending getheaders message : {:?}", e);
            Err(())
        },
    }
}

fn wait_recv_headers(mut conn: Connection) -> Result<(Vec<BlockHeader>, Connection), ()>
{
    match conn.recv_msg() {
        Ok((IncomingMessage::Headers(hs), conn)) => {
            info!("Receive headers message");
            let headers = hs.iter().map(|lone| lone.header).collect();
            Ok((headers, conn))
        },
        Ok((msg, conn)) => {
            info!("Receive unexpected message. Expected headers msg but receive {}", msg);
            info!("Drop connection : {:?}", conn);
            Err(())
        },
        Err(e) => {
            info!("Error while receiving headers message : {:?}", e);
            Err(())
        },
    }
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

fn download_blocks(mut conn: Connection, blockchain: &mut BlockChainMut) -> Result<Connection, ()>
{
    let (n_req_block, conn) = request_getblocks(conn, blockchain)?;
    let conn = wait_recv_blocks_and_apply(conn, blockchain, n_req_block)?;
    if n_req_block == 16 {
        download_blocks(conn, blockchain)
    } else {
        Ok(conn)
    }
}

fn request_getblocks(mut conn: Connection, blockchain: &mut BlockChainMut) -> Result<(usize, Connection), ()>
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
    match conn.send_msg(msg) {
        Ok(conn) => {
            info!("Sent getblocks message");
            Ok((n_invs, conn))
        },
        Err(e) => {
            info!("Error while sending getblocks message : {:?}", e);
            Err(())
        },
    }
}

fn wait_recv_blocks_and_apply(
    mut conn: Connection,
    blockchain: &mut BlockChainMut,
    n_req_block: usize,
) -> Result<Connection, ()>
{
    if n_req_block == 0 {
        return Ok(conn);
    }
    let (block, conn) = match conn.recv_msg() {
        Ok((IncomingMessage::Block(block), conn)) => {
            info!("Receive a new block");
            (block, conn)
        },
        Ok((msg, conn)) => {
            info!("Receive unexpected message. Expected block msg but receive {}", msg);
            info!("Drop connection {:?}", conn);
            return Err(());
        },
        Err(e) => {
            info!("Error while receiving block message : {:?}", e);
            return Err(());
        },
    };
    match blockchain.get_block_mut(block.bitcoin_hash()) {
        Some(b) => *b = BlockData::new_full_block(block),
        None => {
            info!("Receive unexpected block : Hash does not match.");
            return Err(());
        },
    }
    wait_recv_blocks_and_apply(conn, blockchain, n_req_block - 1)
}
