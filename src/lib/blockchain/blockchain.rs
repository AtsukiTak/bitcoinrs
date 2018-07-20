use bitcoin::network::serialize::BitcoinHash;
use bitcoin::util::hash::Sha256dHash;
use std::sync::Arc;

use super::BlockData;

/// A simple implementation of blockchain.
/// This data structure is immutable, in contrast to that `BlockChainMut` is mutable.
/// You can `clone` this with no cost.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockChain
{
    pub(super) blocks: Arc<Vec<BlockData>>,
}

impl BlockChain
{
    /// Get length of current best chain.
    pub fn len(&self) -> usize
    {
        self.blocks.len()
    }

    /// Get iterator representing current best block chain.
    /// Oldest block comes first, latest block comes last.
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a BlockData> + DoubleEndedIterator
    {
        self.blocks.iter()
    }

    /// Get latest block
    ///
    /// The key of this function is `unwrap`; since there are always genesis block at least,
    /// we can call `unwrap`.
    pub fn latest_block(&self) -> &BlockData
    {
        self.iter().rev().next().unwrap() // since there are always genesis block
    }

    pub fn get_block(&self, hash: &Sha256dHash) -> Option<&BlockData>
    {
        self.iter().find(|b| b.bitcoin_hash() == *hash)
    }
}
