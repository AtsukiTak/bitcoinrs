use bitcoin::blockdata::{block::BlockHeader, constants::genesis_block};
use bitcoin::network::{constants::Network, serialize::BitcoinHash};
use bitcoin::util::hash::Sha256dHash;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockData
{
    pub header: BlockHeader,
    pub height: usize,
    hash: Sha256dHash,
}

impl BlockData
{
    pub fn new(header: BlockHeader, height: usize) -> BlockData
    {
        BlockData {
            hash: header.bitcoin_hash(),
            header,
            height,
        }
    }

    pub fn genesis(network: Network) -> BlockData
    {
        BlockData::new(genesis_block(network).header, 0)
    }

    pub fn header(&self) -> &BlockHeader
    {
        &self.header
    }

    pub fn height(&self) -> usize
    {
        self.height
    }
}

impl BitcoinHash for BlockData
{
    fn bitcoin_hash(&self) -> Sha256dHash
    {
        self.hash
    }
}
