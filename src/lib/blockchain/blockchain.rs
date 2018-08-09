use bitcoin::util::hash::Sha256dHash;
use std::sync::Arc;

use super::StoredBlock;

/// A simple implementation of blockchain.
/// This data structure is immutable, in contrast to that `BlockChainMut` is mutable.
/// You can `clone` this with no cost.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockChain<B>
{
    pub(super) blocks: Arc<Vec<B>>,
}

impl<B: StoredBlock> BlockChain<B>
{
    /// Get length of current best chain.
    pub fn len(&self) -> usize
    {
        self.blocks.len()
    }

    /// Get iterator representing current best block chain.
    /// Oldest block comes first, latest block comes last.
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a B> + DoubleEndedIterator
    {
        self.blocks.iter()
    }

    /// Get latest block
    ///
    /// The key of this function is `unwrap`; since there are always genesis block at least,
    /// we can call `unwrap`.
    pub fn latest_block(&self) -> &B
    {
        self.iter().rev().next().unwrap() // since there are always genesis block
    }

    pub fn get_block(&self, hash: &Sha256dHash) -> Option<&B>
    {
        self.iter().find(|b| b.bitcoin_hash() == *hash)
    }
}
