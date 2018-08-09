use bitcoin::blockdata::block::{Block, BlockHeader};
use bitcoin::network::serialize::BitcoinHash;
use bitcoin::util::hash::Sha256dHash;

pub trait StoredBlock: BitcoinHash + Clone
{
    fn new(block: Block) -> Self;

    fn header(&self) -> &BlockHeader;
}

pub trait StoredFullBlock: StoredBlock
{
    fn block(&self) -> &Block;
}

#[derive(Clone, Debug)]
pub struct HeaderOnlyBlock
{
    header: BlockHeader,
    hash: Sha256dHash,
}

impl BitcoinHash for HeaderOnlyBlock
{
    fn bitcoin_hash(&self) -> Sha256dHash
    {
        self.hash
    }
}

impl StoredBlock for HeaderOnlyBlock
{
    fn new(block: Block) -> Self
    {
        HeaderOnlyBlock {
            hash: block.bitcoin_hash(),
            header: block.header,
        }
    }

    fn header(&self) -> &BlockHeader
    {
        &self.header
    }
}
