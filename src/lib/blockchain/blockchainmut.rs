use bitcoin::blockdata::block::{Block, BlockHeader};
use bitcoin::network::serialize::BitcoinHash;
use bitcoin::util::hash::Sha256dHash;

use super::BlockData;

const ENOUGH_CONFIRMATION: usize = 6;

/// A simple implementation of blockchain.
pub struct BlockChainMut
{
    stable_chain: StableBlockChain,
    unstable_chain: UnstableBlockChain,
}

pub struct InvalidBlock;

impl BlockChainMut
{
    /// Creaet a new `BlockChainMut` struct with genesis block.
    /// Note that the term of genesis block in here is not same  with bitcoin's genesis block.
    /// It just root block of returned `BlockChainMut`.
    /// Given genesis block may be different from bitcoin's genesis block.
    /// And also note that given genesis block **MUST** be stable one.
    pub fn with_genesis(block: Block) -> BlockChainMut
    {
        BlockChainMut {
            stable_chain: StableBlockChain::new(),
            unstable_chain: UnstableBlockChain::with_genesis(BlockData::new(block)),
        }
    }

    /// Get length of current best chain.
    pub fn len(&self) -> usize
    {
        self.stable_chain.len() + self.unstable_chain.len()
    }

    /// Try to add a new block.
    /// If success, reference to given block is returned.
    pub fn try_add(&mut self, block: Block) -> Result<&BlockData, InvalidBlock>
    {
        // TODO : Check PoW of given block

        let (stored_block, maybe_stabled) = self.unstable_chain.try_add(BlockData::new(block))?;
        if let Some(stabled) = maybe_stabled {
            self.stable_chain.add_block(stabled);
        }
        Ok(stored_block)
    }

    /// Get iterator representing current best block chain.
    /// Oldest block comes first, latest block comes last.
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a BlockData> + DoubleEndedIterator
    {
        let unstable_blocks = self.unstable_chain.iter();
        let stable_blocks = self.stable_chain.blocks.iter();
        stable_blocks.chain(unstable_blocks)
    }

    /// Get vector representing best block chain.
    /// Oldest block comes first, latest block comes last.
    ///
    /// # Note
    /// This function may be deleted in future because user can just call `self.iter().collect()`.
    pub fn to_vec(&self) -> Vec<&BlockData>
    {
        self.iter().collect()
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

    /// Get locator blocks iterator.
    ///
    /// # Note
    /// Current implementation is **VERY** **VERY** simple.
    /// It should be improved in future.
    /// Bitcoin core's implementation is here.
    /// https://github.com/bitcoin/bitcoin/blob/master/src/chain.cpp#L23
    pub fn locator_blocks<'a>(&'a self) -> impl Iterator<Item = &'a BlockData>
    {
        self.iter().rev().take(10)
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

    fn len(&self) -> usize
    {
        self.blocks.len()
    }

    fn add_block(&mut self, stabled: StabledBlock)
    {
        self.blocks.push(stabled.0);
    }
}

/// Just make sure that given Block is returned by `UnstableBlockChain::try_add_block`.
struct StabledBlock(BlockData);

/// Chain of blocks which is **NOT** confirmed enough.
struct UnstableBlockChain
{
    tree: BlockTree,
}

impl UnstableBlockChain
{
    fn with_genesis(block: BlockData) -> UnstableBlockChain
    {
        UnstableBlockChain {
            tree: BlockTree::with_genesis(block),
        }
    }

    fn len(&self) -> usize
    {
        self.tree.len()
    }

    fn try_add(&mut self, block: BlockData) -> Result<(&BlockData, Option<StabledBlock>), InvalidBlock>
    {
        debug!("Try to add a new block");

        self.tree.try_add(block)
    }

    fn iter(&self) -> BlockTreeIter
    {
        self.tree.iter()
    }
}

struct BlockTree
{
    // Head node. This is almost stabled.
    head: *mut BlockTreeNode,

    // Last node of longest chain
    last: *mut BlockTreeNode,

    // Cache of length
    len: usize,
}

struct BlockTreeNode
{
    prev: Option<*mut BlockTreeNode>,
    nexts: Vec<*mut BlockTreeNode>,
    block: BlockData,

    // Cache to reduce computation
    block_hash: Sha256dHash,
}

impl BlockTree
{
    fn with_genesis(block: BlockData) -> BlockTree
    {
        let node = BlockTreeNode {
            prev: None,
            nexts: vec![],
            block_hash: block.bitcoin_hash(),
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

    fn try_add(&mut self, block: BlockData) -> Result<(&BlockData, Option<StabledBlock>), InvalidBlock>
    {
        unsafe {
            // Search prev block of given block
            let node = find_node_by_hash(self.head, &block.header().prev_blockhash).ok_or(InvalidBlock)?;

            // Append given block to prev node
            let new_node = append_block_to_node(node, block);

            // If new_node is a new tip, replace it
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
            } else {
                // Successfully added a new block but no stabled block is created.
                Ok((stored_block, None))
            }
        }
    }

    fn iter(&self) -> BlockTreeIter
    {
        unsafe { BlockTreeIter::from_last_node(self.last.as_ref().unwrap()) }
    }
}

impl Drop for BlockTree
{
    fn drop(&mut self)
    {
        unsafe { drop_with_sub_node(self.head) };
    }
}

impl BlockTreeNode
{
    fn into_ptr(self) -> *mut BlockTreeNode
    {
        Box::into_raw(Box::new(self))
    }
}

// Make sure `node` is not null
unsafe fn append_block_to_node(node: *mut BlockTreeNode, block: BlockData) -> *mut BlockTreeNode
{
    let new_node = BlockTreeNode {
        prev: Some(node.clone()),
        nexts: vec![],
        block_hash: block.bitcoin_hash(),
        block,
    };
    let new_node_ptr = new_node.into_ptr();
    node.as_mut().unwrap().nexts.push(new_node_ptr.clone());
    new_node_ptr
}

// Serch root node first
// Make sure `node` is not null
unsafe fn find_node_by_hash(node_ptr: *mut BlockTreeNode, hash: &Sha256dHash) -> Option<*mut BlockTreeNode>
{
    let node = node_ptr.as_ref().unwrap();
    if node.block_hash == *hash {
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
unsafe fn depth_from_root(node_ptr: *mut BlockTreeNode) -> usize
{
    let node = node_ptr.as_ref().unwrap();
    if let Some(prev) = node.prev {
        depth_from_root(prev)
    } else {
        0
    }
}

// Make sure `from` is not null
unsafe fn find_prior_node(from: *mut BlockTreeNode, back: usize) -> Option<*mut BlockTreeNode>
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
unsafe fn drop_with_sub_node(node_ptr: *mut BlockTreeNode)
{
    let node = node_ptr.as_ref().unwrap();
    for next in node.nexts.iter() {
        drop_with_sub_node(*next);
    }
    drop(Box::from_raw(node_ptr));
}

pub struct BlockTreeIter<'a>
{
    // to reduce memory allocation, using array with Option instead using Vec
    nodes: [Option<&'a BlockTreeNode>; ENOUGH_CONFIRMATION],
    next: usize,
    next_back: usize,
    finished: bool,
}

impl<'a> BlockTreeIter<'a>
{
    fn from_last_node(last: &'a BlockTreeNode) -> BlockTreeIter<'a>
    {
        let (count, mut prev_nodes) = prev_nodes(last);
        prev_nodes[count] = Some(last);
        BlockTreeIter {
            nodes: prev_nodes,
            next: 0,
            next_back: count,
            finished: false,
        }
    }
}

fn prev_nodes<'a>(node: &'a BlockTreeNode) -> (usize, [Option<&'a BlockTreeNode>; ENOUGH_CONFIRMATION])
{
    if node.prev.is_none() {
        return (0, [None; ENOUGH_CONFIRMATION]);
    }

    let prev = unsafe { node.prev.unwrap().as_ref().unwrap() };
    let (count, mut prev_nodes) = prev_nodes(prev);
    prev_nodes[count] = Some(prev);
    (count, prev_nodes)
}

impl<'a> Iterator for BlockTreeIter<'a>
{
    type Item = &'a BlockData;

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

        Some(&node.block)
    }
}

impl<'a> DoubleEndedIterator for BlockTreeIter<'a>
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

        Some(&node.block)
    }
}
