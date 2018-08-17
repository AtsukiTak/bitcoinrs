use bitcoin::blockdata::block::Block;
use bitcoin::network::serialize::BitcoinHash;
use bitcoin::util::hash::Sha256dHash;

use super::StoredBlock;

pub trait BlockChain
{
    type Block: StoredBlock;

    fn try_add_block(&mut self, block: Block) -> Option<&Self::Block>;

    fn active_chain<'a>(&'a self) -> <&'a Self as IntoActiveChain>::ActiveChain
    where &'a Self: IntoActiveChain
    {
        self.into_active_chain()
    }
}

pub trait IntoActiveChain
{
    type ActiveChain: ActiveChain;

    fn into_active_chain(self) -> Self::ActiveChain;
}

/// Oldest block comes first, latest block comes last.
pub trait ActiveChain
{
    type Block;
    type Iter: Iterator<Item = Self::Block> + DoubleEndedIterator;

    fn len(&self) -> usize;

    fn iter(&self) -> Self::Iter;

    /// Get latest block
    fn latest_block(&self) -> Self::Block
    {
        self.iter().rev().next().expect("No blocks in ActiveChain")
    }

    /// Get specified block
    fn get_block(&self, hash: Sha256dHash) -> Option<Self::Block>
    where Self::Block: BitcoinHash
    {
        self.iter().find(move |b| b.bitcoin_hash() == hash)
    }
}

/// A simple implementation of blockchain.
///  All blocks are active.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinerBlockChain<B>
{
    blocks: Vec<B>,
}

pub struct ActiveLinerBlockChain<'a, B: 'a>
{
    blocks: &'a Vec<B>,
}

impl<B: StoredBlock> LinerBlockChain<B>
{
    pub fn new(blocks: Vec<B>) -> LinerBlockChain<B>
    {
        LinerBlockChain { blocks }
    }
}

impl<B: StoredBlock> BlockChain for LinerBlockChain<B>
{
    type Block = B;

    fn try_add_block(&mut self, block: Block) -> Option<&Self::Block>
    {
        self.blocks.push(block);
        Some(self.blocks.last().unwrap())
    }
}

impl<'a, B> IntoActiveChain for &'a LinerBlockChain<B>
{
    type ActiveChain = ActiveLinerBlockChain<'a, B>;

    fn into_active_chain(self) -> Self::ActiveChain
    {
        ActiveLinerBlockChain { blocks: &self.blcoks }
    }
}

impl<'a, B> ActiveChain for ActiveLinerBlockChain<'a, B>
{
    type Block = &'a B;
    type Iter = ::std::slice::Iter<'a, B>;

    /// Get length of current best chain.
    fn len(&self) -> usize
    {
        self.blocks.len()
    }

    fn iter(&self) -> Self::Iter
    {
        self.blocks.iter()
    }
}
