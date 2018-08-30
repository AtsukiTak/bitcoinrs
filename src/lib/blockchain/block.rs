use bitcoin::blockdata::{block::{Block, BlockHeader}, constants::genesis_block};
use bitcoin::network::{constants::Network, serialize::BitcoinHash};
use bitcoin::util::hash::Sha256dHash;

#[derive(Copy, Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullBlockData
{
    pub block: Block,
    pub height: usize,
    hash: Sha256dHash,
}

impl FullBlockData
{
    pub fn new(block: Block, height: usize) -> FullBlockData
    {
        FullBlockData {
            hash: block.bitcoin_hash(),
            block,
            height,
        }
    }

    pub fn genesis(network: Network) -> FullBlockData
    {
        FullBlockData::new(genesis_block(network), 0)
    }
}

impl BitcoinHash for FullBlockData
{
    fn bitcoin_hash(&self) -> Sha256dHash
    {
        self.hash
    }
}

pub trait BlockDataLike: BitcoinHash
{
    fn header(&self) -> &BlockHeader;
    fn height(&self) -> usize;
}

impl BlockDataLike for BlockData
{
    fn header(&self) -> &BlockHeader
    {
        self.header()
    }

    fn height(&self) -> usize
    {
        self.height()
    }
}

impl BlockDataLike for FullBlockData
{
    fn header(&self) -> &BlockHeader
    {
        &self.block.header
    }

    fn height(&self) -> usize
    {
        self.height
    }
}
