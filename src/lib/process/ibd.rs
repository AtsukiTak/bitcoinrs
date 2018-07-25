use bitcoin::network::{message_blockdata::{GetHeadersMessage, InvType, Inventory}, serialize::BitcoinHash};
use bitcoin::blockdata::block::BlockHeader;
use bitcoin::util::hash::Sha256dHash;

use connection::{Connection, IncomingMessage, OutgoingMessage};
use blockchain::{BlockChainMut, BlockData};

pub fn initial_block_download(
    conn: Connection,
    blockchain: BlockChainMut,
) -> Result<(Connection, BlockChainMut), BlockChainMut>
{
    let mut ibd = IBD { conn, blockchain };
    match ibd.start() {
        Ok(()) => Ok((ibd.conn, ibd.blockchain)),
        Err(()) => Err(ibd.blockchain),
    }
}

struct IBD
{
    conn: Connection,
    blockchain: BlockChainMut,
}

impl IBD
{
    fn start(&mut self) -> Result<(), ()>
    {
        self.download_headers()?;
        self.download_blocks()
    }

    fn download_headers(&mut self) -> Result<(), ()>
    {
        const MAX_HEADERS_IN_MSG: usize = 2000;
        loop {
            self.request_getheaders()?;
            let headers = self.wait_recv_headers()?;
            let n_headers = headers.len();
            self.apply_received_headers(headers)?;
            if n_headers < MAX_HEADERS_IN_MSG {
                break;
            }
        }

        Ok(())
    }

    fn request_getheaders(&mut self) -> Result<(), ()>
    {
        let locator_hashes = self.blockchain.locator_blocks().map(|b| b.bitcoin_hash()).collect();
        let get_headers_msg = GetHeadersMessage::new(locator_hashes, Sha256dHash::default());
        let msg = OutgoingMessage::GetHeaders(get_headers_msg);
        match self.conn.send_msg(msg) {
            Ok(()) => {
                info!("Sent getheaders message");
                Ok(())
            },
            Err(e) => {
                info!("Error while sending getheaders message : {:?}", e);
                Err(())
            },
        }
    }

    fn wait_recv_headers(&mut self) -> Result<Vec<BlockHeader>, ()>
    {
        match self.conn.recv_msg() {
            Ok(IncomingMessage::Headers(hs)) => {
                info!("Receive headers message");
                let headers = hs.iter().map(|lone| lone.header).collect();
                Ok(headers)
            },
            Ok(msg) => {
                info!("Receive unexpected message. Expected headers msg but receive {}", msg);
                Err(())
            },
            Err(e) => {
                info!("Error while receiving headers message : {:?}", e);
                Err(())
            },
        }
    }

    fn apply_received_headers(&mut self, mut headers: Vec<BlockHeader>) -> Result<(), ()>
    {
        for header in headers.drain(..) {
            if let Err(e) = self.blockchain.try_add_header(header) {
                info!("Receive invalid block header. {:?}", e);
                return Err(());
            }
        }
        info!(
            "Applied new headers to internal blockchain. Current length is {}",
            self.blockchain.len()
        );
        Ok(())
    }

    fn download_blocks(&mut self) -> Result<(), ()>
    {
        loop {
            let n_req_block = self.request_getblocks()?;
            self.wait_recv_blocks(n_req_block)?;
            if n_req_block < 16 {
                break;
            }
        }
        Ok(())
    }

    fn request_getblocks(&mut self) -> Result<usize, ()>
    {
        const DL_AT_ONCE_MAX: usize = 16;
        let invs: Vec<_> = self.blockchain
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
        match self.conn.send_msg(msg) {
            Ok(()) => {
                info!("Sent getblocks message");
                Ok(n_invs)
            },
            Err(e) => {
                info!("Error while sending getblocks message : {:?}", e);
                Err(())
            },
        }
    }

    fn wait_recv_blocks(&mut self, n_req_block: usize) -> Result<(), ()>
    {
        for _ in 0..n_req_block {
            let block = match self.conn.recv_msg() {
                Ok(IncomingMessage::Block(block)) => {
                    info!("Receive a new block");
                    block
                },
                Ok(msg) => {
                    info!("Receive unexpected message. Expected block msg but receive {}", msg);
                    return Err(());
                },
                Err(e) => {
                    info!("Error while receiving block message : {:?}", e);
                    return Err(());
                },
            };
            match self.blockchain.get_block_mut(block.bitcoin_hash()) {
                Some(b) => *b = BlockData::new_full_block(block),
                None => {
                    info!("Receive unexpected block : Hash does not match.");
                    return Err(());
                },
            }
        }
        Ok(())
    }
}
