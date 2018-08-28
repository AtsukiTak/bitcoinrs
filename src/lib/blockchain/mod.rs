mod blockchain;
mod blocktree;
mod block;

pub use self::blockchain::BlockChain;
pub use self::blocktree::BlockTree;
pub use self::block::BlockData;

use bitcoin::blockdata::block::BlockHeader;

#[derive(Debug)]
pub struct NotFoundPrevBlock(pub BlockHeader);
