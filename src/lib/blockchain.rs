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

    pub fn try_add(&mut self, block: StoredBlock) -> Result<(), InvalidBlock> {
        // TODO : Check PoW of given block

        if let Some(stabled) = self.unstable_chain.try_add(block)? {
            self.stable_chain.add_block(stabled);
        }
        Ok(())
    }
}

#[derive(Clone)]
pub enum StoredBlock {
    HeaderOnly(BlockHeader),
    FullBlock(Block),
}

impl StoredBlock {
    pub fn header(&self) -> &BlockHeader {
        match self {
            &StoredBlock::HeaderOnly(ref header) => header,
            &StoredBlock::FullBlock(ref block) => &block.header,
        }
    }
}

impl BitcoinHash for StoredBlock {
    fn bitcoin_hash(&self) -> Sha256dHash {
        match self {
            &StoredBlock::HeaderOnly(ref header) => header.bitcoin_hash(),
            &StoredBlock::FullBlock(ref block) => block.bitcoin_hash(),
        }
    }
}

/// Chain of blocks which is confirmed enough.
struct StableBlockChain {
    blocks: Vec<StoredBlock>,
}

impl StableBlockChain {
    pub fn new() -> StableBlockChain {
        StableBlockChain { blocks: Vec::new() }
    }

    pub fn add_block(&mut self, stabled: StabledBlock) {
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

    fn try_add(&mut self, block: StoredBlock) -> Result<Option<StabledBlock>, InvalidBlock> {
        debug!("Try to add a new block");

        self.tree.try_add(block)
    }
}

struct BlockTree {
    head: *mut BlockTreeNode,
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
        BlockTree {
            head: node.into_ptr(),
        }
    }

    fn try_add(&mut self, block: StoredBlock) -> Result<Option<StabledBlock>, InvalidBlock> {
        unsafe {
            // Search prev block of given block
            let node =
                find_node_by_hash(self.head, &block.header().prev_blockhash).ok_or(InvalidBlock)?;

            // Append given block to prev node
            let new_node = append_block_to_node(node, block);

            // If there is node wihch has enough confirmation,
            if let Some(almost_stable) = find_prior_node(new_node, ENOUGH_CONFIRMATION) {
                let stabled_node_ptr = self.head;
                let stabled_node = stabled_node_ptr.as_ref().unwrap();

                // drop outdated nodes
                for next in stabled_node.nexts.iter() {
                    if *next != almost_stable {
                        drop_with_sub_node(*next);
                    }
                }

                // move almost stable node to a new head
                self.head = almost_stable;

                // return head node's block as stabled block
                let block = stabled_node.block.clone();
                drop(Box::from_raw(stabled_node_ptr));
                return Ok(Some(StabledBlock(block)));
            }
        }

        // Successfully added a new block but no stabled block is created.
        Ok(None)
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
