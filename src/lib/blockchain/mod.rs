mod blockchain;
mod blocktree;
mod block;
mod data_manager;

pub use self::blockchain::BlockChain;
pub use self::blocktree::BlockTree;
pub use self::block::{BlockData, BlockDataLike, FullBlockData};
pub use self::data_manager::{BlockAssociatedData, BlockAssociatedDataManager};

use bitcoin::blockdata::block::BlockHeader;

#[derive(Debug)]
pub struct NotFoundPrevBlock(pub BlockHeader);
