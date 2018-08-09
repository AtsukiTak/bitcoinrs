use bitcoin::blockdata::constants::genesis_block;
use bitcoin::network::constants::Network;
use bitcoin::util::hash::Sha256dHash;
use std::{marker::PhantomData, sync::Arc};

use super::{BlockChain, StoredBlock};

const ENOUGH_CONFIRMATION: usize = 12;

/// A simple implementation of blockchain.
pub struct BlockChainMut<B>
{
    stable_chain: StableBlockChain<B>,
    unstable_chain: UnstableBlockChain<B>,
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
            unstable_chain: UnstableBlockChain::with_start(block),
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

    /// Get immutable `BlockChain`.
    pub fn freeze(&self) -> BlockChain<B>
    {
        BlockChain {
            blocks: Arc::new(self.iter().cloned().collect()),
        }
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

/// Just make sure that given Block is returned by `UnstableBlockChain::try_add_block`.
struct StabledBlock<B>(B);

/// Chain of blocks which is **NOT** confirmed enough.
struct UnstableBlockChain<B>
{
    tree: BlockTree<B>,
}

impl<B: StoredBlock> UnstableBlockChain<B>
{
    fn with_start(block: B) -> UnstableBlockChain<B>
    {
        UnstableBlockChain {
            tree: BlockTree::with_start(block),
        }
    }

    fn len(&self) -> usize
    {
        self.tree.len()
    }

    fn try_add(&mut self, block: B) -> Result<(&B, Option<StabledBlock<B>>), InvalidBlock>
    {
        self.tree.try_add(block)
    }

    fn iter<'a>(&'a self) -> impl Iterator<Item = &'a B> + DoubleEndedIterator
    {
        self.tree
            .iter()
            .map(|node_ptr| unsafe { &node_ptr.as_ref().unwrap().block })
    }

    fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item = &'a mut B> + DoubleEndedIterator
    {
        self.tree
            .iter()
            .map(|node_ptr| unsafe { &mut node_ptr.as_mut().unwrap().block })
    }
}

struct BlockTree<B>
{
    // Head node. This is almost stabled.
    head: *mut BlockTreeNode<B>,

    // Last node of longest chain
    last: *mut BlockTreeNode<B>,

    // Cache of length
    len: usize,
}

#[derive(Debug)]
struct BlockTreeNode<B>
{
    prev: Option<*mut BlockTreeNode<B>>,
    nexts: Vec<*mut BlockTreeNode<B>>,
    block: B,
}

impl<B: StoredBlock> BlockTree<B>
{
    fn with_start(block: B) -> BlockTree<B>
    {
        let node = BlockTreeNode {
            prev: None,
            nexts: vec![],
            block,
        };
        let node_ptr = node.into_ptr();

        BlockTree {
            head: node_ptr,
            last: node_ptr,
            len: 1,
        }
    }

    fn len(&self) -> usize
    {
        self.len
    }

    fn try_add(&mut self, block: B) -> Result<(&B, Option<StabledBlock<B>>), InvalidBlock>
    {
        unsafe {
            // Search prev block of given block
            let node = find_node_by_hash(self.head, &block.header().prev_blockhash).ok_or(InvalidBlock)?;

            // Append given block to prev node
            let new_node = append_block_to_node(node, block);

            // If new_node is a new tip, replace
            let old_tip_depth = depth_from_root(self.last);
            let new_node_depth = depth_from_root(new_node);
            if old_tip_depth < new_node_depth {
                self.last = new_node;
            }
            self.len += 1;

            // Note that `stored_block`'s lifetime is same with `self`
            let stored_block = &new_node.as_ref().unwrap().block;

            // If there is node wihch has enough confirmation,
            if let Some(enough_confirmed) = find_prior_node(new_node, ENOUGH_CONFIRMATION) {
                // Node which is confirmed enough becomes next head.
                // And head get stabled.
                if enough_confirmed.as_ref().unwrap().prev == Some(self.head) {
                    self.len -= 1;

                    let stabled_node_ptr = self.head;
                    let stabled_node = stabled_node_ptr.as_ref().unwrap();

                    // drop outdated nodes
                    for next in stabled_node.nexts.iter() {
                        if *next != enough_confirmed {
                            drop_with_sub_node(*next);
                        }
                    }

                    // move almost stable node to a new head
                    enough_confirmed.as_mut().unwrap().prev = None;
                    self.head = enough_confirmed;

                    // return head node's block as stabled block
                    let block = stabled_node.block.clone();
                    drop(Box::from_raw(stabled_node_ptr));
                    return Ok((stored_block, Some(StabledBlock(block))));
                } else if enough_confirmed == self.head {
                    Ok((stored_block, None))
                } else {
                    panic!("Never comes here!!");
                }
            } else {
                // Successfully added a new block but no stabled block is created.
                Ok((stored_block, None))
            }
        }
    }

    fn iter(&self) -> BlockTreeIter<B>
    {
        unsafe { BlockTreeIter::from_last_node(self.last) }
    }
}

impl<B> Drop for BlockTree<B>
{
    fn drop(&mut self)
    {
        unsafe { drop_with_sub_node(self.head) };
    }
}

impl<B> BlockTreeNode<B>
{
    fn into_ptr(self) -> *mut BlockTreeNode<B>
    {
        Box::into_raw(Box::new(self))
    }
}

// Make sure `node` is not null
unsafe fn append_block_to_node<B: StoredBlock>(node: *mut BlockTreeNode<B>, block: B) -> *mut BlockTreeNode<B>
{
    let new_node = BlockTreeNode {
        prev: Some(node.clone()),
        nexts: vec![],
        block,
    };
    let new_node_ptr = new_node.into_ptr();
    node.as_mut().unwrap().nexts.push(new_node_ptr.clone());
    new_node_ptr
}

// Serch root node first
// Make sure `node` is not null
unsafe fn find_node_by_hash<B: StoredBlock>(
    node_ptr: *mut BlockTreeNode<B>,
    hash: &Sha256dHash,
) -> Option<*mut BlockTreeNode<B>>
{
    let node = node_ptr.as_ref().unwrap();
    if node.block.bitcoin_hash() == *hash {
        return Some(node_ptr);
    }

    // Search child nodes
    for next in node.nexts.iter() {
        if let Some(node) = find_node_by_hash(*next, hash) {
            return Some(node);
        }
    }

    None
}

// Make sure `node` is not null
unsafe fn depth_from_root<B: StoredBlock>(node_ptr: *mut BlockTreeNode<B>) -> usize
{
    let node = node_ptr.as_ref().unwrap();
    if let Some(prev) = node.prev {
        1 + depth_from_root(prev)
    } else {
        0
    }
}

// Make sure `from` is not null
unsafe fn find_prior_node<B: StoredBlock>(from: *mut BlockTreeNode<B>, back: usize) -> Option<*mut BlockTreeNode<B>>
{
    if back == 0 {
        return Some(from);
    }
    match from.as_ref().unwrap().prev {
        Some(prev) => find_prior_node(prev, back - 1),
        None => None,
    }
}

// Make sure `node_ptr` is not null.
unsafe fn drop_with_sub_node<B>(node_ptr: *mut BlockTreeNode<B>)
{
    let node = node_ptr.as_ref().unwrap();
    for next in node.nexts.iter() {
        drop_with_sub_node(*next);
    }
    drop(Box::from_raw(node_ptr));
}

struct BlockTreeIter<'a, B: 'a>
{
    // to reduce memory allocation, using array with Option instead using Vec
    nodes: [Option<*mut BlockTreeNode<B>>; ENOUGH_CONFIRMATION + 1],
    _lifetime: PhantomData<&'a BlockTreeNode<B>>,
    next: usize,
    next_back: usize,
    finished: bool,
}

impl<'a, B: StoredBlock> BlockTreeIter<'a, B>
{
    // Make sure `last` is not null
    unsafe fn from_last_node(last: *mut BlockTreeNode<B>) -> BlockTreeIter<'a, B>
    {
        let (count, mut prev_nodes) = prev_nodes(last);
        prev_nodes[count] = Some(last);
        BlockTreeIter {
            nodes: prev_nodes,
            _lifetime: PhantomData,
            next: 0,
            next_back: count,
            finished: false,
        }
    }
}

// Make sure that `node_ptr` is not null
unsafe fn prev_nodes<B: StoredBlock>(
    node_ptr: *mut BlockTreeNode<B>,
) -> (usize, [Option<*mut BlockTreeNode<B>>; ENOUGH_CONFIRMATION + 1])
{
    let node = node_ptr.as_ref().unwrap();
    if node.prev.is_none() {
        return (0, [None; ENOUGH_CONFIRMATION + 1]);
    }

    // Get prev node's prev nodes.
    let prev_node_ptr = node.prev.unwrap();
    let (count, mut prev_prev_nodes) = prev_nodes(prev_node_ptr);
    prev_prev_nodes[count] = Some(prev_node_ptr);
    (count + 1, prev_prev_nodes)
}

impl<'a, B> Iterator for BlockTreeIter<'a, B>
{
    type Item = *mut BlockTreeNode<B>;

    fn next(&mut self) -> Option<Self::Item>
    {
        if self.finished {
            return None;
        }

        if self.next == self.next_back {
            // finish at this call
            self.finished = true;
        }

        let node = self.nodes[self.next].unwrap();

        self.next += 1;

        Some(node)
    }
}

impl<'a, B> DoubleEndedIterator for BlockTreeIter<'a, B>
{
    fn next_back(&mut self) -> Option<Self::Item>
    {
        if self.finished {
            return None;
        }

        let node = self.nodes[self.next_back].unwrap();

        if self.next == self.next_back {
            // finish at this call
            self.finished = true;
        } else {
            self.next_back -= 1;
        }

        Some(node)
    }
}

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
    fn blocktree_try_add()
    {
        let start_block_header = dummy_block_header(Sha256dHash::default());
        let next_block_header = dummy_block_header(start_block_header.bitcoin_hash());
        let mut blocktree = BlockTree::with_start(BlockData::new(start_block_header.clone()));

        blocktree.try_add(BlockData::new(next_block_header.clone())).unwrap(); // Should success.

        assert_eq!(blocktree.len(), 2);

        let headers: Vec<_> = blocktree
            .iter()
            .map(|node| unsafe { (*node).block.header().clone() })
            .collect();
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
