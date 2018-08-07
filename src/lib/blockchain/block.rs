use bitcoin::blockdata::block::BlockHeader;
use bitcoin::network::serialize::BitcoinHash;
use bitcoin::util::hash::Sha256dHash;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockData
{
    header: Arc<BlockHeader>,
    hash: Sha256dHash, // Just for cache.
}

impl BlockData
{
    pub fn new(header: BlockHeader) -> BlockData
    {
        BlockData {
            hash: header.bitcoin_hash(),
            header: Arc::new(header),
        }
    }

    pub fn header(&self) -> &BlockHeader
    {
        &self.header
    }
}

impl BitcoinHash for BlockData
{
    fn bitcoin_hash(&self) -> Sha256dHash
    {
        self.hash
    }
}
