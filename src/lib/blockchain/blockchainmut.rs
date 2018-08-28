use bitcoin::blockdata::block::BlockHeader;
use bitcoin::network::serialize::BitcoinHash;
use bitcoin::util::hash::Sha256dHash;

use super::{blocktree, BlockData, BlockTree, NotFoundPrevBlock};

const DEFAULT_ENOUGH_CONF: usize = 100;

/// A hybrid implementation of blockchain.
/// The performance is higher than `BlockTree`.
/// To achieve such performance, this implementation is based on tiny assumption;
/// the block which has enough confirmation will never be changed.
pub struct BlockChainMut
{
    stable_chain: StableBlockChain,

    unstable_chain: BlockTree,

    // The number of confirmation needed to become stable.
    enough_confirmation: usize,
}

impl BlockChainMut
{
    /// Creaet a new `BlockChainMut` struct with start block.
    /// Note that given start block **MUST** be stable one.
    ///
    /// # Panic
    /// if a length of `blocks` is 0.
    pub fn with_initial(blocks: Vec<BlockData>) -> BlockChainMut
    {
        assert!(blocks.len() > 0);

        BlockChainMut {
            stable_chain: StableBlockChain::new(),
            unstable_chain: BlockTree::with_initial(blocks),
            enough_confirmation: DEFAULT_ENOUGH_CONF,
        }
    }

    /// Sets the `enough_confirmation` field.
    pub fn set_enough_confirmation(&mut self, conf: usize)
    {
        self.enough_confirmation = conf;
    }

    /// Try to add a new block.
    pub fn try_add(&mut self, block_header: BlockHeader) -> Result<(), NotFoundPrevBlock>
    {
        self.unstable_chain.try_add(block_header)?;

        while self.unstable_chain.active_chain().len() > self.enough_confirmation {
            let stabled_block = self.unstable_chain.pop_head_unchecked();
            self.stable_chain.add_block(stabled_block);
        }

        Ok(())
    }

    pub fn active_chain(&self) -> ActiveChain
    {
        ActiveChain {
            stabled: self.stable_chain.as_vec(),
            unstabled: self.unstable_chain.active_chain(),
        }
    }
}

impl ::std::fmt::Debug for BlockChainMut
{
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error>
    {
        write!(f, "BlockChainMut{{ .. }}")
    }
}

/// Chain of blocks which is confirmed enough.
struct StableBlockChain
{
    blocks: Vec<BlockData>,
}

impl StableBlockChain
{
    fn new() -> StableBlockChain
    {
        StableBlockChain { blocks: Vec::new() }
    }

    fn add_block(&mut self, block: BlockData)
    {
        self.blocks.push(block);
    }

    fn as_vec(&self) -> &Vec<BlockData>
    {
        &self.blocks
    }
}

pub struct ActiveChain<'a>
{
    stabled: &'a Vec<BlockData>,
    unstabled: blocktree::ActiveChain<'a>,
}

impl<'a> ActiveChain<'a>
{
    pub fn len(&self) -> usize
    {
        self.stabled.len() + self.unstabled.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &BlockData> + DoubleEndedIterator
    {
        let stabled_iter = self.stabled.iter();
        let unstabled_iter = self.unstabled.iter();
        stabled_iter.chain(unstabled_iter)
    }

    /// Get latest block
    ///
    /// The key of this function is `unwrap`; since there are always start block at least,
    /// we can call `unwrap`.
    pub fn latest_block(&self) -> &BlockData
    {
        self.iter().rev().next().unwrap() // since there are always start block
    }

    /// Get block whose hash is exactly same with given hash.
    pub fn get_block(&self, hash: Sha256dHash) -> Option<&BlockData>
    {
        self.iter().find(move |b| b.bitcoin_hash() == hash)
    }

    /// Get locator blocks iterator.
    ///
    /// # Note
    /// Current implementation is **VERY** **VERY** simple.
    /// It should be improved in future.
    /// Bitcoin core's implementation is here.
    /// https://github.com/bitcoin/bitcoin/blob/master/src/chain.cpp#L23
    pub fn locator_blocks(&self) -> impl Iterator<Item = &BlockData>
    {
        self.iter().rev().take(10)
    }
}

/// TODO: Should test re-org case
#[cfg(test)]
mod tests
{
    use super::*;
    use blockchain::{HeaderOnlyBlockData, RawBlockData};
    use bitcoin::blockdata::block::{Block, BlockHeader};
    use bitcoin::network::serialize::BitcoinHash;

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
        let start_block = HeaderOnlyBlockData::new(start_block_header, 0);
        let next_block = Block {
            header: next_block_header,
            txdata: Vec::new(),
        };
        let mut blockchain = BlockChainMut::with_initial(vec![start_block], |raw: RawBlockData| {
            HeaderOnlyBlockData::new(raw.block.header, raw.height)
        });

        assert_eq!(blockchain.active_chain().len(), 1);

        blockchain.try_add(next_block).unwrap(); // Should success.

        assert_eq!(blockchain.active_chain().len(), 2);

        let active_chain = blockchain.active_chain();
        let headers: Vec<_> = active_chain.iter().map(|b| b.header().clone()).collect();
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

        let block1_data = HeaderOnlyBlockData::new(block1, 0);
        let block2 = Block {
            header: block2,
            txdata: Vec::new(),
        };
        let block3 = Block {
            header: block3,
            txdata: Vec::new(),
        };
        let block4 = Block {
            header: block4,
            txdata: Vec::new(),
        };
        let block5 = Block {
            header: block5,
            txdata: Vec::new(),
        };
        let block6 = Block {
            header: block6,
            txdata: Vec::new(),
        };
        let block7 = Block {
            header: block7,
            txdata: Vec::new(),
        };
        let block8 = Block {
            header: block8,
            txdata: Vec::new(),
        };

        let mut blockchain = BlockChainMut::with_initial(vec![block1_data], |raw: RawBlockData| {
            HeaderOnlyBlockData::new(raw.block.header, raw.height)
        });
        blockchain.set_enough_confirmation(7);

        blockchain.try_add(block2).unwrap();
        blockchain.try_add(block3).unwrap();
        blockchain.try_add(block4).unwrap();
        blockchain.try_add(block5).unwrap();
        blockchain.try_add(block6).unwrap();
        blockchain.try_add(block7).unwrap();
        blockchain.try_add(block8).unwrap();

        assert_eq!(blockchain.stable_chain.blocks.len(), 1);
        assert_eq!(blockchain.unstable_chain.active_chain().len(), 7);
    }
}
