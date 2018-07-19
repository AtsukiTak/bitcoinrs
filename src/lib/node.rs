use bitcoin::network::{message::NetworkMessage, message_blockdata::{GetBlocksMessage, InvType, Inventory},
                       serialize::BitcoinHash};
use bitcoin::util::hash::Sha256dHash;
use bitcoin::blockdata::block::Block;

use connection::Connection;
use blockchain::{BlockChain, StoredBlock};

pub struct Node
{
    blockchain: BlockChain,
}

pub enum ProcessResult
{
    Ack,
    Ban,
}

impl Node
{
    /// Send `getblocks` message to given `peer`.
    /// When we start, we need to send `getblocks` message first and then,
    /// we receive `inv` message as response.
    pub fn request_blocks(&self, peer: &mut Connection) -> ProcessResult
    {
        let locator_hashes = self.blockchain.locator_blocks().map(|b| b.bitcoin_hash()).collect();
        let get_blocks_msg = GetBlocksMessage::new(locator_hashes, Sha256dHash::default());
        let network_msg = NetworkMessage::GetBlocks(get_blocks_msg);
        if peer.send_msg(network_msg).is_err() {
            ProcessResult::Ban
        } else {
            ProcessResult::Ack
        }
    }

    /// Process incoming `inv` message.
    /// `inv` message often be sent as response of `getblocks` message.
    /// After we receive `inv` message, we send `getdata` message.
    pub fn recv_inv(&self, invs: Vec<Inventory>, peer: &mut Connection) -> ProcessResult
    {
        // Check received invs all are valid.
        if !check_invs(invs.as_slice(), &self.blockchain) {
            warn!("Peer {:?} send us unwanted inventory. So we disconnect.", peer);
            return ProcessResult::Ban;
        }

        self.request_data(invs, peer)
    }

    /// Send `getdata` message to given `peer`.
    fn request_data(&self, invs: Vec<Inventory>, peer: &mut Connection) -> ProcessResult
    {
        let msg = NetworkMessage::GetData(invs);
        if peer.send_msg(msg).is_err() {
            ProcessResult::Ban
        } else {
            ProcessResult::Ack
        }
    }

    pub fn recv_block(&mut self, block: Block, peer: &mut Connection) -> ProcessResult
    {
        info!("Process incoming block");
        match self.blockchain.try_add(StoredBlock::new(block)) {
            Ok(_) => ProcessResult::Ack,
            Err(_) => {
                warn!("Peer {:?} send us unwanted block. So we disconnect.", peer);
                ProcessResult::Ban
            },
        }
    }
}

fn check_invs(invs: &[Inventory], blockchain: &BlockChain) -> bool
{
    for inv in invs.iter() {
        // Check inventory's type.
        // Should we accept `WitnessBlock` as well?.
        if inv.inv_type != InvType::Block {
            warn!("Incoming inventory's type is not  Block but {:?}", inv.inv_type);
            return false;
        }

        // Check whether given inventory is already stored.
        if blockchain.get_block(&inv.hash).is_some() {
            warn!("Incoming inventory is already stored.");
            return false;
        }
    }

    true
}
