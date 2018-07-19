use bitcoin::network::{message::NetworkMessage, message_blockdata::Inventory,
                       serialize::BitcoinHash};

use connection::Connection;
use blockchain::BlockChain;
use error::Error;

pub struct Node {
    blockchain: BlockChain,
}

impl Node {
    fn recv_inv(&mut self, invs: Vec<Inventory>, peer: &mut Connection) {
        // TODO
    }

    /// Send `GetBlocks` message to given `peer`.
    fn request_blocks(&self, peer: &mut Connection) {
        // ordered newest to oldest.
        // For now, I use very naive implementation. Need to fix later.
        let blocks = self.blockchain.iter();
        let n_blocks = self.blockchain.len();
        let locator_blocks = self.blockchain.iter().skip(n_blocks - 10);
        let locator_hashes = locator_blocks.map(|b| b.bitcoin_hash());
    }

    fn request_data(&self, invs: Vec<Inventory>, peer: &mut Connection) -> Result<(), Error> {
        let msg = NetworkMessage::GetData(invs);
        peer.send_msg(msg)
    }
}
