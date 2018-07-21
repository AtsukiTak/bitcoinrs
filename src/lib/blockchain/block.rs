use bitcoin::blockdata::block::{Block, BlockHeader};
use bitcoin::network::serialize::BitcoinHash;
use bitcoin::util::hash::Sha256dHash;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockData
{
    block: Arc<Block>,
    hash: Sha256dHash, // Just for cache.
}

impl BlockData
{
    pub fn new(block: Block) -> BlockData
    {
        BlockData {
            hash: block.bitcoin_hash(),
            block: Arc::new(block),
        }
    }

    pub fn header(&self) -> &BlockHeader
    {
        &self.block.header
    }

    pub fn block(&self) -> &Block
    {
        &self.block
    }
}

impl BitcoinHash for BlockData
{
    fn bitcoin_hash(&self) -> Sha256dHash
    {
        self.hash
    }
}
