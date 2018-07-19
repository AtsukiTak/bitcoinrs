use bitcoin::network::{message::NetworkMessage, message_blockdata::{GetBlocksMessage, InvType, Inventory},
                       serialize::BitcoinHash};
use bitcoin::util::hash::Sha256dHash;

use connection::Connection;
use blockchain::BlockChain;
use error::Error;

pub struct Node
{
    blockchain: BlockChain,
}

impl Node
{
    /// Process incoming `inv` message.
    /// `inv` message often be sent as response of `getblocks` message.
    /// After we receive `inv` message, we send `getdata` message.
    fn recv_inv(&mut self, invs: Vec<Inventory>, peer: &mut Connection)
    {
        if !check_invs(invs.as_slice(), &self.blockchain) {
            warn!("Peer {:?} send us unwanted inventory. So we disconnect.", peer);
            peer.disconnect();
            return;
        }

        for inv in invs.iter() {
            info!("Process incoming inventory : {:?}", inv);
        }
    }

    /// Send `getblocks` message to given `peer`.
    /// When we start, we need to send `getblocks` message first and then,
    /// we receive `inv` message as response.
    fn request_blocks(&self, peer: &mut Connection) -> Result<(), Error>
    {
        let locator_hashes = self.blockchain.locator_blocks().map(|b| b.bitcoin_hash()).collect();
        let get_blocks_msg = GetBlocksMessage::new(locator_hashes, Sha256dHash::default());
        let network_msg = NetworkMessage::GetBlocks(get_blocks_msg);
        peer.send_msg(network_msg)
    }

    fn request_data(&self, invs: Vec<Inventory>, peer: &mut Connection) -> Result<(), Error>
    {
        let msg = NetworkMessage::GetData(invs);
        peer.send_msg(msg)
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
