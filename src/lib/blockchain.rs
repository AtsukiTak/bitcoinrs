use bitcoin::blockdata::block::{Block, BlockHeader};
use bitcoin::network::serialize::BitcoinHash;
use bitcoin::util::hash::Sha256dHash;

const ENOUGH_CONFIRMATION: usize = 6;

/// A simple implementation of blockchain.
pub struct BlockChain {
    stable_chain: StableBlockChain,
    unstable_chain: UnstableBlockChain,
}

pub struct InvalidBlock;

impl BlockChain {
    pub fn with_genesis(block: StoredBlock) -> BlockChain {
        BlockChain {
            stable_chain: StableBlockChain::new(),
            unstable_chain: UnstableBlockChain::with_genesis(block),
        }
    }

    pub fn len(&self) -> usize {
        self.stable_chain.len() + self.unstable_chain.len()
    }

    pub fn try_add(&mut self, block: StoredBlock) -> Result<&StoredBlock, InvalidBlock> {
        // TODO : Check PoW of given block

        let (stored_block, maybe_stabled) = self.unstable_chain.try_add(block)?;
        if let Some(stabled) = maybe_stabled {
            self.stable_chain.add_block(stabled);
        }
        Ok(stored_block)
    }

    pub fn try_add_header(&mut self, header: BlockHeader) -> Result<&StoredBlock, InvalidBlock> {
        self.try_add(StoredBlock::header_only(header))
    }

    pub fn try_add_full_block(&mut self, block: Block) -> Result<&StoredBlock, InvalidBlock> {
        self.try_add(StoredBlock::full_block(block))
    }

    // Genesis block is first
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a StoredBlock> + DoubleEndedIterator {
        let unstable_blocks = self.unstable_chain.iter();
        let stable_blocks = self.stable_chain.blocks.iter();
        stable_blocks.chain(unstable_blocks)
    }

    pub fn to_vec(&self) -> Vec<&StoredBlock> {
        self.iter().collect()
    }

    pub fn latest_block(&self) -> &StoredBlock {
        self.iter().rev().next().unwrap() // since there always genesis block
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StoredBlock {
    HeaderOnly(BlockHeader),
    FullBlock(Block),
}

impl StoredBlock {
    pub fn header_only(header: BlockHeader) -> StoredBlock {
        StoredBlock::HeaderOnly(header)
    }

    pub fn full_block(block: Block) -> StoredBlock {
        StoredBlock::FullBlock(block)
    }

    pub fn header(&self) -> &BlockHeader {
        match self {
            &StoredBlock::HeaderOnly(ref header) => header,
            &StoredBlock::FullBlock(ref block) => &block.header,
        }
    }
}

impl BitcoinHash for StoredBlock {
    fn bitcoin_hash(&self) -> Sha256dHash {
        self.header().bitcoin_hash()
    }
}

/// Chain of blocks which is confirmed enough.
struct StableBlockChain {
    blocks: Vec<StoredBlock>,
}

impl StableBlockChain {
    fn new() -> StableBlockChain {
        StableBlockChain { blocks: Vec::new() }
    }

    fn len(&self) -> usize {
        self.blocks.len()
    }

    fn add_block(&mut self, stabled: StabledBlock) {
        self.blocks.push(stabled.0);
    }
}

/// Just make sure that given Block is returned by `UnstableBlockChain::try_add_block`.
struct StabledBlock(StoredBlock);

/// Chain of blocks which is **NOT** confirmed enough.
struct UnstableBlockChain {
    tree: BlockTree,
}

impl UnstableBlockChain {
    fn with_genesis(block: StoredBlock) -> UnstableBlockChain {
        UnstableBlockChain {
            tree: BlockTree::with_genesis(block),
        }
    }

    fn len(&self) -> usize {
        self.tree.len()
    }

    fn try_add(
        &mut self,
        block: StoredBlock,
    ) -> Result<(&StoredBlock, Option<StabledBlock>), InvalidBlock> {
        debug!("Try to add a new block");

        self.tree.try_add(block)
    }

    fn iter(&self) -> BlockTreeIter {
        self.tree.iter()
    }
}

struct BlockTree {
    // Head node. This is almost stabled.
    head: *mut BlockTreeNode,

    // Last node of longest chain
    last: *mut BlockTreeNode,

    // Cache of length
    len: usize,
}

struct BlockTreeNode {
    prev: Option<*mut BlockTreeNode>,
    nexts: Vec<*mut BlockTreeNode>,
    block: StoredBlock,

    // Cache to reduce computation
    block_hash: Sha256dHash,
}

impl BlockTree {
    fn with_genesis(block: StoredBlock) -> BlockTree {
        let node = BlockTreeNode {
            prev: None,
            nexts: vec![],
            block_hash: block.bitcoin_hash(),
            block: block,
        };
        let node_ptr = node.into_ptr();

        BlockTree {
            head: node_ptr,
            last: node_ptr,
            len: 1,
        }
    }

    fn len(&self) -> usize {
        self.len
    }

    fn try_add(
        &mut self,
        block: StoredBlock,
    ) -> Result<(&StoredBlock, Option<StabledBlock>), InvalidBlock> {
        unsafe {
            // Search prev block of given block
            let node =
                find_node_by_hash(self.head, &block.header().prev_blockhash).ok_or(InvalidBlock)?;

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

    fn iter(&self) -> BlockTreeIter {
        unsafe { BlockTreeIter::from_last_node(self.last.as_ref().unwrap()) }
    }
}

impl Drop for BlockTree {
    fn drop(&mut self) {
        unsafe { drop_with_sub_node(self.head) };
    }
}

impl BlockTreeNode {
    fn into_ptr(self) -> *mut BlockTreeNode {
        Box::into_raw(Box::new(self))
    }
}

// Make sure `node` is not null
unsafe fn append_block_to_node(node: *mut BlockTreeNode, block: StoredBlock) -> *mut BlockTreeNode {
    let new_node = BlockTreeNode {
        prev: Some(node.clone()),
        nexts: vec![],
        block_hash: block.bitcoin_hash(),
        block: block,
    };
    let new_node_ptr = new_node.into_ptr();
    node.as_mut().unwrap().nexts.push(new_node_ptr.clone());
    new_node_ptr
}

// Serch root node first
// Make sure `node` is not null
unsafe fn find_node_by_hash(
    node_ptr: *mut BlockTreeNode,
    hash: &Sha256dHash,
) -> Option<*mut BlockTreeNode> {
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
unsafe fn depth_from_root(node_ptr: *mut BlockTreeNode) -> usize {
    let node = node_ptr.as_ref().unwrap();
    if let Some(prev) = node.prev {
        depth_from_root(prev)
    } else {
        0
    }
}

// Make sure `from` is not null
unsafe fn find_prior_node(from: *mut BlockTreeNode, back: usize) -> Option<*mut BlockTreeNode> {
    if back == 0 {
        return Some(from);
    }
    match from.as_ref().unwrap().prev {
        Some(prev) => find_prior_node(prev, back - 1),
        None => None,
    }
}

// Make sure `node_ptr` is not null.
unsafe fn drop_with_sub_node(node_ptr: *mut BlockTreeNode) {
    let node = node_ptr.as_ref().unwrap();
    for next in node.nexts.iter() {
        drop_with_sub_node(*next);
    }
    drop(Box::from_raw(node_ptr));
}

pub struct BlockTreeIter<'a> {
    // to reduce memory allocation, using array with Option instead using Vec
    nodes: [Option<&'a BlockTreeNode>; ENOUGH_CONFIRMATION],
    next: usize,
    next_back: usize,
    finished: bool,
}

impl<'a> BlockTreeIter<'a> {
    fn from_last_node(last: &'a BlockTreeNode) -> BlockTreeIter<'a> {
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

fn prev_nodes<'a>(
    node: &'a BlockTreeNode,
) -> (usize, [Option<&'a BlockTreeNode>; ENOUGH_CONFIRMATION]) {
    if node.prev.is_none() {
        return (0, [None; ENOUGH_CONFIRMATION]);
    }

    let prev = unsafe { node.prev.unwrap().as_ref().unwrap() };
    let (count, mut prev_nodes) = prev_nodes(prev);
    prev_nodes[count] = Some(prev);
    (count, prev_nodes)
}

impl<'a> Iterator for BlockTreeIter<'a> {
    type Item = &'a StoredBlock;

    fn next(&mut self) -> Option<Self::Item> {
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

impl<'a> DoubleEndedIterator for BlockTreeIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        if self.next == self.next_back {
            // finish at this call
            self.finished = true;
        }

        let node = self.nodes[self.next_back].unwrap();

        self.next_back -= 1;

        Some(&node.block)
    }
}
