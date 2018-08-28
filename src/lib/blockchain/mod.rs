mod blockchainmut;
// mod blockchain;
mod blocktree;
mod block;

pub use self::blockchainmut::BlockChainMut;
pub use self::blocktree::BlockTree;
pub use self::block::BlockData;

use bitcoin::blockdata::block::BlockHeader;

#[derive(Debug)]
pub struct NotFoundPrevBlock(pub BlockHeader);
