use bitcoin::util::hash::Sha256dHash;
use bitcoin::blockdata::block::BlockHeader;
use bitcoin::network::{constants::Network, serialize::BitcoinHash};
use std::ptr::NonNull;

use super::{BlockData, NotFoundPrevBlock};


/// A honest implementation of blockchain.
pub struct BlockTree
{
    // Nodes of current active chain
    active_nodes: Vec<NonNull<Node>>,
}

#[derive(Debug)]
struct Node
{
    prev: Option<NonNull<Node>>,
    nexts: Vec<NonNull<Node>>,
    block: BlockData,
}

impl BlockTree
{
    pub fn new(network: Network) -> BlockTree
    {
        BlockTree::with_start(BlockData::genesis(network))
    }

    pub fn with_start(block_data: BlockData) -> BlockTree
    {
        let node = Node::new(block_data);
        BlockTree {
            active_nodes: vec![node],
        }
    }

    /// # Note
    /// Does not check blockchain validity
    ///
    /// # Panic
    /// if a length of `blocks` is 0.
    pub fn with_initial(blocks: Vec<BlockData>) -> BlockTree
    {
        assert!(blocks.len() > 0);

        let mut nodes: Vec<NonNull<Node>> = blocks.into_iter().map(Node::new).collect();

        {
            // updaet `prev` field
            let nodes_cloned = nodes.clone();
            let nodes_skip_first = nodes.iter_mut().skip(1);
            for (node, prev) in nodes_skip_first.zip(nodes_cloned) {
                unsafe { node.as_mut().prev = Some(prev) };
            }
        }

        {
            // update `nexts` field
            let nodes_skip_first = nodes.clone().into_iter().skip(1);
            for (node, next) in nodes.iter_mut().zip(nodes_skip_first) {
                unsafe { node.as_mut().nexts.push(next) };
            }
        }

        BlockTree { active_nodes: nodes }
    }

    pub fn try_add(&mut self, block_header: BlockHeader) -> Result<(), NotFoundPrevBlock>
    {
        /* Defines some useful function */

        // Returns last common `Node` between `active_chain` and `node_ptr`'s branch.
        fn find_last_common(active_chain: ActiveChain, node_ptr: NonNull<Node>) -> NonNull<Node>
        {
            let node = unsafe { node_ptr.as_ref() };
            if active_chain.contains(&node.block) {
                return node_ptr;
            }
            match node.prev {
                None => unreachable!(), // because independent branch never exist.
                Some(prev) => find_last_common(active_chain, prev),
            }
        }

        // Rewinded `active_chain` contains a node whose height is `rewind_height`.
        // Length of `active_chain` must be long enough.
        fn rewind_active_chain(active_chain: &mut Vec<NonNull<Node>>, rewind_height: usize)
        {
            unsafe {
                let start_height = active_chain[0].as_ref().block.height();
                let rewind_idx = rewind_height - start_height + 1;
                active_chain.set_len(rewind_idx);
            }
        }

        fn append_nodes(active_chain: &mut Vec<NonNull<Node>>, node_ptr: NonNull<Node>)
        {
            unsafe {
                let node = node_ptr.as_ref();
                let prev_node = node.prev.unwrap();
                if prev_node != *active_chain.last().unwrap() {
                    append_nodes(active_chain, prev_node);
                }
                active_chain.push(node_ptr);
            }
        }

        /* logic starts from here */

        // Search prev block of given block
        let prev_node = match self.find_node(block_header.prev_blockhash) {
            None => return Err(NotFoundPrevBlock(block_header)),
            Some(node) => node,
        };

        // Generates `BlockData`.
        let prev_block_height = unsafe { prev_node.as_ref().block.height() };
        let new_block_height = prev_block_height + 1;
        let new_block_data = BlockData::new(block_header, new_block_height);

        // Creates a new node
        let new_node = Node::append_block(prev_node, new_block_data);

        // If new_node is a new tip, replace
        let tail_block_height = unsafe { self.active_nodes.last().unwrap().as_ref().block.height() };
        if tail_block_height < new_block_height {
            // Rewinds current active chain
            let last_common_node = find_last_common(self.active_chain(), new_node);
            let rewind_height = unsafe { last_common_node.as_ref().block.height() };
            rewind_active_chain(&mut self.active_nodes, rewind_height);
            append_nodes(&mut self.active_nodes, new_node);
        }

        Ok(())
    }

    /// Find a block whose bitcoin_hash is equal to given hash
    /// It is depth first search.
    fn find_node(&self, hash: Sha256dHash) -> Option<NonNull<Node>>
    {
        fn inner(node_ptr: NonNull<Node>, hash: Sha256dHash) -> Option<NonNull<Node>>
        {
            let node = unsafe { node_ptr.as_ref() };

            for next in node.nexts.iter() {
                if let Some(node) = inner(*next, hash) {
                    return Some(node);
                }
            }

            if node.block.bitcoin_hash() == hash {
                return Some(node_ptr);
            }

            None
        }

        inner(self.active_nodes[0], hash)
    }

    pub fn active_chain(&self) -> ActiveChain
    {
        ActiveChain {
            nodes: &self.active_nodes,
        }
    }

    /// Pop head block.
    ///
    /// # Panic
    /// if only one block is contained.
    pub fn pop_head_unchecked(&mut self) -> BlockData
    {
        let poped_head = self.active_nodes.remove(0);
        let mut next_head = self.active_nodes[0]; // panic if length is 1.

        // Drop nodes which will be dangling.
        for may_drop_node in unsafe { poped_head.as_ref().nexts.iter() } {
            if *may_drop_node != next_head {
                unsafe { drop_with_sub_node(*may_drop_node) };
            }
        }

        unsafe {
            next_head.as_mut().prev = None;
        }

        unsafe { Node::into_block(Box::from_raw(poped_head.as_ptr())) }
    }
}

impl Drop for BlockTree
{
    fn drop(&mut self)
    {
        unsafe { drop_with_sub_node(self.active_nodes[0]) };
    }
}

impl Node
{
    fn new(block: BlockData) -> NonNull<Node>
    {
        let new_node = Node {
            prev: None,
            nexts: vec![],
            block,
        };
        unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(new_node))) }
    }

    fn append_block(mut node: NonNull<Node>, block: BlockData) -> NonNull<Node>
    {
        let new_node = Node {
            prev: Some(node.clone()),
            nexts: vec![],
            block,
        };
        let new_node_ptr = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(new_node))) };

        // lifetime is valid
        unsafe { node.as_mut().nexts.push(new_node_ptr.clone()) };

        new_node_ptr
    }

    fn into_block(self: Box<Self>) -> BlockData
    {
        self.block
    }
}

unsafe fn drop_with_sub_node(node_ptr: NonNull<Node>)
{
    for next in node_ptr.as_ref().nexts.iter() {
        drop_with_sub_node(*next);
    }
    drop(Box::from_raw(node_ptr.as_ptr()));
}

pub struct ActiveChain<'a>
{
    // TODO : Need non-alocation way
    nodes: &'a Vec<NonNull<Node>>,
}

impl<'a> ActiveChain<'a>
{
    pub fn len(&self) -> usize
    {
        self.nodes.len()
    }

    pub fn get_block(&self, height: usize) -> Option<&BlockData>
    {
        let start_height = self.iter().next().unwrap().height();
        if start_height < height {
            return None;
        }
        self.nodes
            .get(height - start_height)
            .map(|p| unsafe { &p.as_ref().block })
    }

    pub fn contains(&self, block: &BlockData) -> bool
    {
        self.get_block(block.height()).is_some()
    }

    pub fn iter(&self) -> impl Iterator<Item = &BlockData> + DoubleEndedIterator
    {
        self.nodes.iter().map(|node| unsafe { &node.as_ref().block })
    }
}

/// TODO: Should test re-org case
#[cfg(test)]
mod tests
{
    use super::*;
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
    fn blocktree_try_add()
    {
        let start_block_header = dummy_block_header(Sha256dHash::default());
        let next_block_header = dummy_block_header(start_block_header.bitcoin_hash());
        let start_block = BlockData::new(start_block_header, 0);
        let mut blocktree = BlockTree::with_start(start_block);

        assert_eq!(blocktree.active_chain().len(), 1);

        blocktree.try_add(next_block_header).unwrap(); // Should success.

        assert_eq!(blocktree.active_chain().len(), 2);

        let active_chain = blocktree.active_chain();
        let headers: Vec<_> = active_chain.iter().map(|block| block.header).collect();
        assert_eq!(headers, vec![start_block_header, next_block_header]);
    }
}
