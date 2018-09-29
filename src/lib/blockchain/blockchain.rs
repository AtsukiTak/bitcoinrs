use std::{cell::{Ref, RefCell}, collections::VecDeque, sync::{Arc, Weak}};

use bitcoin::util::hash::Sha256dHash;
use bitcoin::blockdata::block::BlockHeader;
use bitcoin::network::{constants::Network, serialize::BitcoinHash};

use super::{BlockData, NotFoundPrevBlock};


/// A honest implementation of blockchain.
pub struct BlockChain
{
    // Nodes of current active chain
    active_nodes: VecDeque<Arc<RefCell<Node>>>,
}

pub struct ActiveChain<'a>
{
    nodes: &'a VecDeque<Arc<RefCell<Node>>>,
}

impl BlockChain
{
    pub fn new(network: Network) -> BlockChain
    {
        BlockChain::with_start(BlockData::genesis(network))
    }

    pub fn with_start(block_data: BlockData) -> BlockChain
    {
        let node = Node::new(block_data);
        let mut vec = VecDeque::new();
        vec.push_back(node);
        BlockChain { active_nodes: vec }
    }

    pub fn try_add(&mut self, block_header: BlockHeader) -> Result<(), NotFoundPrevBlock>
    {
        self.try_add_inner(block_header)
    }

    pub fn active_chain(&self) -> ActiveChain
    {
        ActiveChain {
            nodes: &self.active_nodes,
        }
    }
}

impl<'a> ActiveChain<'a>
{
    pub fn len(&self) -> u32
    {
        self.nodes.len() as u32
    }

    /// Get the latest block
    ///
    /// Note that there always be latest block.
    pub fn latest_block<'b>(&'b self) -> Ref<BlockData>
    {
        self.iter().rev().next().unwrap()
    }

    /// Get the specified height block
    pub fn get_block<'b>(&'b self, height: u32) -> Option<Ref<'b, BlockData>>
    {
        let start_height = self.iter().next().unwrap().height;
        if height < start_height {
            return None;
        }
        self.nodes
            .get((height - start_height) as usize)
            .map(|node| Ref::map(node.as_ref().borrow(), |n| &n.block))
    }

    /// Check whether active chain contains given block or not.
    pub fn contains(&self, block: &BlockData) -> bool
    {
        match self.get_block(block.height) {
            None => false,
            Some(b) => b.bitcoin_hash() == block.bitcoin_hash(),
        }
    }

    pub fn iter<'b>(&'b self) -> impl Iterator<Item = Ref<'b, BlockData>> + DoubleEndedIterator
    {
        self.nodes
            .iter()
            .map(|node| Ref::map(node.as_ref().borrow(), |n| &n.block))
    }


    /// Get locator block's hash iterator.
    ///
    /// # Note
    /// Current implementation is **VERY** **VERY** simple.
    /// It should be improved in future.
    /// Bitcoin core's implementation is here.
    /// https://github.com/bitcoin/bitcoin/blob/master/src/chain.cpp#L23
    pub fn locator_hashes<'b>(&'b self) -> impl Iterator<Item = Sha256dHash> + 'b
    {
        // TODO improve this algo
        self.iter().rev().take(10).map(|b| b.bitcoin_hash())
    }

    /// Get locator block's hash vec.
    pub fn locator_hashes_vec(&self) -> Vec<Sha256dHash>
    {
        let mut vec = Vec::with_capacity(10);
        for hash in self.locator_hashes() {
            vec.push(hash);
        }
        vec
    }
}

impl BlockChain
{
    fn try_add_inner(&mut self, block_header: BlockHeader) -> Result<(), NotFoundPrevBlock>
    {
        /* logic starts from here */

        // Search prev block of given block
        let prev_node = match self.borrow_then_find_node(block_header.prev_blockhash) {
            None => return Err(NotFoundPrevBlock(block_header)),
            Some(node) => node,
        };

        // Generates `BlockData`.
        let prev_block_height = {
            // immutable borrow start
            prev_node.borrow().block.height()
            // immutable borrow end
        };
        let new_block_height = prev_block_height + 1;
        let new_block_data = BlockData::new(block_header, new_block_height);

        // Append a new block to back of `prev_node`.
        let new_node = Node::borrow_mut_then_append_block(&prev_node, new_block_data);

        // If new_node is a new tip, replace
        let tail_block_height = {
            // immutable borrow start
            self.active_nodes.back().unwrap().borrow().block.height()
            // immutable borrow end
        };
        if tail_block_height < new_block_height {
            // Rewinds current active chain
            let last_common_node = self.borrow_then_find_last_common(&new_node);
            let rewind_height = {
                // immutable borrow start
                last_common_node.borrow().block.height()
                // immutable borrow end
            };
            self.borrow_then_rewind_active_chain(rewind_height);
            self.borrow_then_append_nodes(new_node);
        }

        Ok(())
    }

    // Returns last common `Node` between `active_chain` and `node_ptr`'s branch.
    fn borrow_then_find_last_common(&self, node_ptr: &Arc<RefCell<Node>>) -> Arc<RefCell<Node>>
    {
        fn inner(active_chain: ActiveChain, node_ptr: &Arc<RefCell<Node>>) -> Arc<RefCell<Node>>
        {
            let node = node_ptr.borrow();
            if active_chain.contains(&node.block) {
                return node_ptr.clone();
            }
            match Node::borrow_then_get_prev(node_ptr) {
                None => unreachable!(), // because independent branch never exist.
                Some(prev) => inner(active_chain, &prev),
            }
        }

        inner(self.active_chain(), node_ptr)
    }

    // # Note
    // Rewinded `active_chain` contains a node whose height is `rewind_height`.
    // Length of `active_chain` **MUST** be long enough.
    fn borrow_then_rewind_active_chain(&mut self, rewind_height: u32)
    {
        let start_height = self.active_nodes[0].borrow().block.height();
        let rewind_idx = rewind_height - start_height + 1;
        self.active_nodes.truncate(rewind_idx as usize);
    }

    /// Append nodes of given `node_ptr`'s branch.
    /// # Note
    /// The last active node **MUST** be on `node_ptr`'s branch.
    fn borrow_then_append_nodes(&mut self, node_ptr: Arc<RefCell<Node>>)
    {
        match Node::borrow_then_get_prev(&node_ptr) {
            None => panic!("node_ptr must have prev node"),
            Some(prev_node) => {
                if !Arc::ptr_eq(&prev_node, self.active_nodes.back().unwrap()) {
                    self.borrow_then_append_nodes(prev_node);
                }
                // Now, `prev_node == active_chain.back().unwrap()`
                self.active_nodes.push_back(node_ptr);
            },
        }
    }

    /// Find a block whose bitcoin_hash is equal to given hash
    /// Depth first search.
    fn borrow_then_find_node(&self, hash: Sha256dHash) -> Option<Arc<RefCell<Node>>>
    {
        fn inner(node_ptr: &Arc<RefCell<Node>>, hash: Sha256dHash) -> Option<Arc<RefCell<Node>>>
        {
            let node = node_ptr.borrow();

            // Depth first search
            for next in node.nexts.iter() {
                if let Some(node) = inner(next, hash) {
                    return Some(node);
                }
            }

            if node.block.bitcoin_hash() == hash {
                return Some(node_ptr.clone());
            }

            None
        }

        inner(&self.active_nodes[0], hash)
    }
}

#[derive(Debug)]
/// Node may be strongly referenced from
///
/// 1. parent node as `next` node
/// 2. BlockChain as `active` node
///
/// During one of these reference alive, Node never be dropped.
///
/// So if `self.prev.unwrap().upgrade()` returns `None`,
/// it means that above two reference does not alive,
/// i.e. self is head node.
struct Node
{
    prev: Weak<RefCell<Node>>,
    nexts: Vec<Arc<RefCell<Node>>>,
    block: BlockData,
}

impl Node
{
    fn new(block: BlockData) -> Arc<RefCell<Node>>
    {
        let new_node = Node {
            prev: Weak::new(),
            nexts: vec![],
            block,
        };
        Arc::new(RefCell::new(new_node))
    }

    /// # Note
    /// Inside this function, `node.borrow_mut()` is called.
    /// So caller **MUTS** take care of not calling `node.borrow_mut()` in parent scope.
    fn borrow_mut_then_append_block(node: &Arc<RefCell<Node>>, block: BlockData) -> Arc<RefCell<Node>>
    {
        let new_node = Node {
            prev: Arc::downgrade(node),
            nexts: vec![],
            block,
        };
        let new_node_ptr = Arc::new(RefCell::new(new_node));

        node.borrow_mut().nexts.push(new_node_ptr.clone());

        new_node_ptr
    }

    fn borrow_then_get_prev(node: &Arc<RefCell<Node>>) -> Option<Arc<RefCell<Node>>>
    {
        node.borrow().prev.upgrade()
    }
}

/// TODO: Should test re-org case
#[cfg(test)]
mod tests
{
    use super::*;

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
    fn blocktree_try_add()
    {
        let start_block_header = dummy_block_header(Sha256dHash::default());
        let next_block_header = dummy_block_header(start_block_header.bitcoin_hash());
        let start_block = BlockData::new(start_block_header, 0);
        let mut blocktree = BlockChain::with_start(start_block);

        assert_eq!(blocktree.active_chain().len(), 1);

        blocktree.try_add(next_block_header).unwrap(); // Should success.

        assert_eq!(blocktree.active_chain().len(), 2);

        let active_chain = blocktree.active_chain();
        let headers: Vec<_> = active_chain.iter().map(|block| block.header).collect();
        assert_eq!(headers, vec![start_block_header, next_block_header]);
    }
}
