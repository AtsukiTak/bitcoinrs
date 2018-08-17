use bitcoin::blockdata::constants::genesis_block;
use bitcoin::network::constants::Network;
use bitcoin::util::hash::Sha256dHash;
use std::sync::Arc;

use super::{BlockTree, StoredBlock};

const ENOUGH_CONFIRMATION: usize = 12;

/// A simple implementation of blockchain.
pub struct BlockChainMut<B>
{
    stable_chain: StableBlockChain<B>,
    unstable_chain: BlockTree<B>,
}

#[derive(Debug)]
pub struct InvalidBlock;

impl<B: StoredBlock> BlockChainMut<B>
{
    /// Create a new `BlockChainMut` struct with main net genesis block.
    /// If you want another network (such as test network) genesis block,
    /// please use `with_start` function.
    pub fn new() -> BlockChainMut<B>
    {
        BlockChainMut::with_start(B::new(genesis_block(Network::Bitcoin)))
    }

    /// Creaet a new `BlockChainMut` struct with start block.
    /// Note that given start block **MUST** be stable one.
    pub fn with_start(block: B) -> BlockChainMut<B>
    {
        BlockChainMut {
            stable_chain: StableBlockChain::new(),
            unstable_chain: BlockTree::with_start(block),
        }
    }

    /// Get length of current best chain.
    pub fn len(&self) -> usize
    {
        self.stable_chain.len() + self.unstable_chain.len()
    }

    /// Try to add a new block.
    /// If success, reference to given block is returned.
    pub fn try_add(&mut self, block: B) -> Result<&B, InvalidBlock>
    {
        // TODO : Check PoW of given block

        let (stored_block, maybe_stabled) = self.unstable_chain.try_add(block)?;
        if let Some(stabled) = maybe_stabled {
            self.stable_chain.add_block(stabled);
        }
        Ok(stored_block)
    }

    /// Get iterator representing current best block chain.
    /// Oldest block comes first, latest block comes last.
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a B> + DoubleEndedIterator
    {
        let unstable_blocks = self.unstable_chain.iter();
        let stable_blocks = self.stable_chain.blocks.iter();
        stable_blocks.chain(unstable_blocks)
    }

    pub fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item = &'a mut B> + DoubleEndedIterator
    {
        let unstable_blocks = self.unstable_chain.iter_mut();
        let stable_blocks = self.stable_chain.blocks.iter_mut();
        stable_blocks.chain(unstable_blocks)
    }

    /// Get vector representing best block chain.
    /// Oldest block comes first, latest block comes last.
    ///
    /// # Note
    /// Is there better way to create `Vec`?
    pub fn to_vec(&self) -> Vec<&B>
    {
        self.iter().collect()
    }

    /// Get latest block
    ///
    /// The key of this function is `unwrap`; since there are always start block at least,
    /// we can call `unwrap`.
    pub fn latest_block(&self) -> &B
    {
        self.iter().rev().next().unwrap() // since there are always start block
    }

    /// Get block whose hash is exactly same with given hash.
    pub fn get_block(&self, hash: Sha256dHash) -> Option<&B>
    {
        self.iter().find(move |b| b.bitcoin_hash() == hash)
    }

    pub fn get_block_mut(&mut self, hash: Sha256dHash) -> Option<&mut B>
    {
        self.iter_mut().find(move |b| b.bitcoin_hash() == hash)
    }

    /// Get locator blocks iterator.
    ///
    /// # Note
    /// Current implementation is **VERY** **VERY** simple.
    /// It should be improved in future.
    /// Bitcoin core's implementation is here.
    /// https://github.com/bitcoin/bitcoin/blob/master/src/chain.cpp#L23
    pub fn locator_blocks<'a>(&'a self) -> impl Iterator<Item = &'a B>
    {
        self.iter().rev().take(10)
    }
}

impl<B> ::std::fmt::Debug for BlockChainMut<B>
{
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error>
    {
        write!(f, "BlockChainMut{{ .. }}")
    }
}

/// Chain of blocks which is confirmed enough.
struct StableBlockChain<B>
{
    blocks: Vec<B>,
}

impl<B: StoredBlock> StableBlockChain<B>
{
    fn new() -> StableBlockChain<B>
    {
        StableBlockChain { blocks: Vec::new() }
    }

    fn len(&self) -> usize
    {
        self.blocks.len()
    }

    fn add_block(&mut self, stabled: StabledBlock<B>)
    {
        self.blocks.push(stabled.0);
    }
}

/// Just make sure that given Block is returned by `BlockTree::try_add_block`.
pub struct StabledBlock<B>(pub B);

/// TODO: Should test re-org case
#[cfg(test)]
mod tests
{
    use super::*;
    use bitcoin::blockdata::block::{Block, BlockHeader};

    fn dummy_block_header(prev_hash: Sha256dHash) -> BlockHeader
    {
        let header = BlockHeader {
            version: 1,
            prev_blockhash: prev_hash,
            merkle_root: Sha256dHash::default(),
            time: 0,
            bits: 0,
            nonce: 0,
        };
        header
    }

    #[test]
    fn blockchainmut_try_add()
    {
        let start_block_header = dummy_block_header(Sha256dHash::default());
        let next_block_header = dummy_block_header(start_block_header.bitcoin_hash());
        let mut blockchain = BlockChainMut::with_start(BlockData::new(start_block_header.clone()));

        blockchain.try_add(BlockData::new(next_block_header.clone())).unwrap(); // Should success.

        assert_eq!(blockchain.len(), 2);

        let headers: Vec<_> = blockchain.iter().map(|b| b.header().clone()).collect();
        assert_eq!(headers, vec![start_block_header, next_block_header]);
    }

    #[test]
    fn add_8_blocks_to_blockchainmut()
    {
        let block1 = dummy_block_header(Sha256dHash::default());
        let block2 = dummy_block_header(block1.bitcoin_hash());
        let block3 = dummy_block_header(block2.bitcoin_hash());
        let block4 = dummy_block_header(block3.bitcoin_hash());
        let block5 = dummy_block_header(block4.bitcoin_hash());
        let block6 = dummy_block_header(block5.bitcoin_hash());
        let block7 = dummy_block_header(block6.bitcoin_hash());
        let block8 = dummy_block_header(block7.bitcoin_hash());

        let mut blockchain = BlockChainMut::with_start(BlockData::new(block1));

        blockchain.try_add(BlockData::new(block2)).unwrap();
        blockchain.try_add(BlockData::new(block3)).unwrap();
        blockchain.try_add(BlockData::new(block4)).unwrap();
        blockchain.try_add(BlockData::new(block5)).unwrap();
        blockchain.try_add(BlockData::new(block6)).unwrap();
        blockchain.try_add(BlockData::new(block7)).unwrap();
        blockchain.try_add(BlockData::new(block8)).unwrap();

        assert_eq!(blockchain.stable_chain.len(), 1);
        assert_eq!(blockchain.unstable_chain.len(), 7);
    }
}
