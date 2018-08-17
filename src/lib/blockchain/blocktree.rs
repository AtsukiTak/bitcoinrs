use bitcoin::util::hash::Sha256dHash;
use std::marker::PhantomData;

use super::{InvalidBlock, StabledBlock, StoredBlock};

const ENOUGH_CONFIRMATION: usize = 12;

pub struct BlockTree<B>
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
    pub fn with_start(block: B) -> BlockTree<B>
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

    pub fn len(&self) -> usize
    {
        self.len
    }

    pub fn try_add(&mut self, block: B) -> Result<(&B, Option<StabledBlock<B>>), InvalidBlock>
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

    pub fn iter(&self) -> impl Iterator<Item = &B> + DoubleEndedIterator
    {
        unsafe { BlockTreeIter::from_last_node(self.last).map(|node_ptr| &node_ptr.as_ref().unwrap().block) }
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut B> + DoubleEndedIterator
    {
        unsafe { BlockTreeIter::from_last_node(self.last).map(|node_ptr| &mut node_ptr.as_mut().unwrap().block) }
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
