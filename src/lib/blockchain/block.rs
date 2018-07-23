use bitcoin::blockdata::block::{Block, BlockHeader};
use bitcoin::network::serialize::BitcoinHash;
use bitcoin::util::hash::Sha256dHash;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockData
{
    block: InnerBlock,
    hash: Sha256dHash, // Just for cache.
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum InnerBlock
{
    HeaderOnly(Arc<BlockHeader>),
    FullBlock(Arc<Block>),
}

impl BlockData
{
    pub fn new_header_only(header: BlockHeader) -> BlockData
    {
        BlockData {
            hash: header.bitcoin_hash(),
            block: InnerBlock::new_header_only(header),
        }
    }

    pub fn new_full_block(block: Block) -> BlockData
    {
        BlockData {
            hash: block.bitcoin_hash(),
            block: InnerBlock::new_full_block(block),
        }
    }

    pub fn header(&self) -> &BlockHeader
    {
        &self.block.header()
    }

    pub fn full_block(&self) -> Option<&Block>
    {
        self.block.full_block()
    }

    pub fn is_header_only(&self) -> bool
    {
        self.full_block().is_none()
    }

    pub fn is_full_block(&self) -> bool
    {
        self.full_block().is_some()
    }
}

impl BitcoinHash for BlockData
{
    fn bitcoin_hash(&self) -> Sha256dHash
    {
        self.hash
    }
}

impl InnerBlock
{
    fn new_header_only(header: BlockHeader) -> InnerBlock
    {
        InnerBlock::HeaderOnly(Arc::new(header))
    }

    fn new_full_block(block: Block) -> InnerBlock
    {
        InnerBlock::FullBlock(Arc::new(block))
    }

    fn header(&self) -> &BlockHeader
    {
        match self {
            &InnerBlock::HeaderOnly(ref h) => h,
            &InnerBlock::FullBlock(ref b) => &b.header,
        }
    }

    fn full_block(&self) -> Option<&Block>
    {
        match self {
            &InnerBlock::HeaderOnly(_) => None,
            &InnerBlock::FullBlock(ref b) => Some(b),
        }
    }
}
