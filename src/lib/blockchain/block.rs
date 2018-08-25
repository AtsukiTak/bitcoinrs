use bitcoin::blockdata::block::{Block, BlockHeader};
use bitcoin::network::serialize::BitcoinHash;
use bitcoin::util::hash::Sha256dHash;

/*  Trait definition */

pub trait BlockData: BitcoinHash
{
    fn height(&self) -> usize;

    fn header(&self) -> &BlockHeader;
}

pub trait FullBlockData: BlockData
{
    fn block(&self) -> &Block;
}

pub trait BlockGenerator
{
    type BlockData: BlockData;

    fn generate_block(&mut self, block: RawBlockData) -> Self::BlockData;
}

/*  BlockData definition */

#[derive(Debug)]
pub struct RawBlockData
{
    block: Block,
    height: usize,
    hash: Sha256dHash,
}

impl RawBlockData
{
    pub fn new(block: Block, height: usize) -> RawBlockData
    {
        RawBlockData {
            hash: block.bitcoin_hash(),
            block,
            height,
        }
    }
}

impl BitcoinHash for RawBlockData
{
    fn bitcoin_hash(&self) -> Sha256dHash
    {
        self.hash
    }
}

impl BlockData for RawBlockData
{
    fn height(&self) -> usize
    {
        self.height
    }

    fn header(&self) -> &BlockHeader
    {
        &self.block.header
    }
}

impl FullBlockData for RawBlockData
{
    fn block(&self) -> &Block
    {
        &self.block
    }
}

#[derive(Debug)]
pub struct HeaderOnlyBlockData
{
    header: BlockHeader,
    height: usize,
    hash: Sha256dHash,
}

impl HeaderOnlyBlockData
{
    pub fn new(header: BlockHeader, height: usize) -> HeaderOnlyBlockData
    {
        HeaderOnlyBlockData {
            hash: header.bitcoin_hash(),
            height,
            header,
        }
    }
}

impl BitcoinHash for HeaderOnlyBlockData
{
    fn bitcoin_hash(&self) -> Sha256dHash
    {
        self.hash
    }
}

impl BlockData for HeaderOnlyBlockData
{
    fn header(&self) -> &BlockHeader
    {
        &self.header
    }

    fn height(&self) -> usize
    {
        self.height
    }
}

/*  BlockGenerator definition */

pub struct DefaultBlockGenerator;

impl BlockGenerator for DefaultBlockGenerator
{
    type BlockData = RawBlockData;

    fn generate_block(&mut self, block: RawBlockData) -> Self::BlockData
    {
        block
    }
}

impl<B, G> BlockGenerator for G
where
    G: FnMut(RawBlockData) -> B,
    B: BlockData,
{
    type BlockData = B;

    fn generate_block(&mut self, block: RawBlockData) -> Self::BlockData
    {
        self(block)
    }
}
