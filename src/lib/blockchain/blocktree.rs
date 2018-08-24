use bitcoin::util::hash::Sha256dHash;
use std::marker::PhantomData;
use std::ptr::NonNull;

use super::{BlockData, BlockGenerator, DefaultBlockGenerator, InvalidBlock, RawBlockData, StabledBlock};

const ENOUGH_CONFIRMATION: usize = 12;

pub struct BlockTree<B, G>
{
    // Head node.
    head: NonNull<Node<B>>,

    // Last node of longest chain
    tail: NonNull<Node<B>>,

    // Nodes of current active chain
    active_nodes: Vec<NonNull<Node<B>>>,

    // Generator which is used when a new block is added
    block_generator: G,
}

#[derive(Debug)]
struct Node<B>
{
    prev: Option<NonNull<Node<B>>>,
    nexts: Vec<NonNull<Node<B>>>,
    block: B,
}

pub struct NotFoundPrevBlock(pub Block);

impl<B, G> BlockTree<B, G>
where
    B: BlockData,
    G: BlockGenerator<BlockData = B>,
{
    /// # Note
    /// Does not check blockchain validity
    pub fn with_initial(blocks: Vec<B>) -> BlockTree<B>
    {
        let mut nodes: Vec<_> = blocks.into_iter().map(Node::new);

        // updaet `prev` field
        let nodes_cloned = nodes.iter().cloned();
        let nodes_skip_first = nodes.iter_mut().skip(1);
        for (node, prev) in nodes_skip_first.zip(nodes_cloned) {
            node.prev = Some(prev);
        }

        // update `nexts` field
        let nodes_skip_first = nodes.iter().cloned().skip(1);
        let nodes = nodes.iter_mut();
        for (node, next) in nodes.zip(nodes_skip_first) {
            node.nexts.push(next);
        }

        BlockTree {
            head: nodes.first().unwrap(),
            tail: nodes.last().unwrap(),
            len: nodes.len(),
            active_nodes: nodes,
            block_generator: DefaultBlockGenerator,
        }
    }

    pub fn try_add(&mut self, block: Block) -> Result<&B, NotFoundPrevBlock>
    {
        fn find_fork<B>(active_chain: ActiveChain<B>, block: NonNull<Node<B>>) -> Option<NonNull<Node<B>>>
        {
        }
        // Search prev block of given block
        let node = match self.depth_first_search(block.header().prev_blockhash) {
            None => return Err(NotFoundPrevBlock(block)),
            Some(node) => node,
        };

        // Generates `BlockData`.
        let prev_block = unsafe { node.as_ref().block };
        let new_block_height = prev_block.height() + 1;
        let raw_block_data = RawBlockData {
            hash: block.bitcoin_hash(),
            block,
            height: new_block_height,
        };
        let block_data = self.block_generator.generate_block(raw_block_data);

        // Creates a new node
        let new_node = Node::append_block(node, block);

        // If new_node is a new tip, replace
        let tail_block_height = unsafe { self.tail.as_ref().block.height() };
        if tail_block_height < new_block_height {
            // find fork point
            let active_chain = self.active_chain();
            let mut fork_point = old_tip;
            unsafe {
                while !active_chain.contains(fork_point.as_ref().block) {
                    fork_point = fork_point.as_ref().prev;
                }
            }
        }

        Ok(unsafe { new_node.as_ref().block })
    }

    fn depth_first_search(&self, hash: Sha256dHash) -> Option<NonNull<Node<B>>>
    {
        fn inner<B: BlockData>(node_ptr: NonNull<Node<B>>, hash: Sha256dHash) -> Option<NonNull<Node<B>>>
        {
            let node = unsafe { node_ptr.as_ref() };

            for next in node.nexts.iter() {
                if let Some(node) = inner(next, hash) {
                    return Some(node);
                }
            }

            if node.block.bitcoin_hash() == hash {
                return Some(node_ptr);
            }

            None
        }

        let head_ptr = self.head?;
        inner(head_ptr, hash)
    }

    pub fn active_chain(&self) -> ActiveChain
    {
        let nodes = {
            match self.tail {
                None => Vec::new(),
                Some(tail) => {
                    let mut vec = Node::prev_nodes(tail);
                    vec.push(tail);
                    vec
                },
            }
        };
        ActiveChain {
            nodes,
            _lifetime: PhantomData,
            next: 0,
            next_back: nodes.len(),
            finished: false,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = B> + DoubleEndedIterator
    {
        self.active_chain().iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut B> + DoubleEndedIterator
    {
        unsafe { BlockTreeIter::from_last_node(self.tail).map(|node_ptr| &mut node_ptr.as_mut().unwrap().block) }
    }
}

impl<B> Drop for BlockTree<B>
{
    fn drop(&mut self)
    {
        unsafe { drop_with_sub_node(self.head) };
    }
}

impl<B> Node<B>
{
    fn new(block: B) -> NonNull<Node<B>>
    {
        let new_node = Node {
            prev: None,
            nexts: vec![],
            block,
        };
        NonNull::new_unchecked(Box::into_raw(Box::new(new_node)))
    }

    fn append_block(node: NonNull<Node<B>>, block: B) -> NonNull<Node<B>>
    {
        let new_node = Node {
            prev: Some(node.clone()),
            nexts: vec![],
            block,
        };
        let new_node_ptr = NonNull::new_unchecked(Box::into_raw(Box::new(new_node)));

        // lifetime is valid
        unsafe { node.as_mut().nexts.push(new_node_ptr.clone()) };

        new_node_ptr
    }

    fn depth_from_root(node: NonNull<Node<B>>) -> usize
    {
        let node = unsafe { node.as_ref() };
        match node.prev {
            Some(prev) => 1 + Node::depth_from_root(prev),
            None => 0,
        }
    }

    fn prev_nodes(node: NonNull<Node<B>>) -> Vec<NonNull<Node<B>>>
    {
        match node.prev {
            None => Vec::new(),
            Some(prev) => {
                let mut vec = Node::prev_nodes(prev);
                vec.push(prev);
                vec
            },
        }
    }

    fn into_block(self: Box<Self>) -> B
    {
        self.block
    }
}

/*
// Make sure `from` is not null
unsafe fn find_prior_node<B: BlockData>(from: *mut Node<B>, back: usize) -> Option<*mut Node<B>>
{
    if back == 0 {
        return Some(from);
    }
    match from.as_ref().unwrap().prev {
        Some(prev) => find_prior_node(prev, back - 1),
        None => None,
    }
}
*/

unsafe fn drop_with_sub_node<B>(node_ptr: *mut Node<B>)
{
    let node = node_ptr.as_ref().unwrap();
    for next in node.nexts.iter() {
        drop_with_sub_node(*next);
    }
    drop(Box::from_raw(node_ptr));
}

pub struct ActiveChain<'a, B: 'a>
{
    // TODO : Need non-alocation way
    nodes: &'a Vec<NonNull<Node<B>>>,
}

impl<'a, B: BlockData> ActiveChain<'a, B>
{
    pub fn len(&self) -> usize
    {
        self.nodes.len()
    }

    pub fn get_block(&self, height: usize) -> Option<&B>
    {
        let start_height = unsafe { self.nodes.first()?.block.height() };
        if start_height < height {
            return None;
        }
        self.nodes.get(height - start_height)
    }

    pub fn contains(&self, b: B) -> bool
    {
        self.get_block(b.height()).is_some()
    }

    pub fn iter(&self) -> ::std::slice::Iter<B>
    {
        self.nodes.iter()
    }
}

impl<'a, B> Iterator for ActiveChainIter<'a, B>
{
    type Item = Node<B>;

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
}
