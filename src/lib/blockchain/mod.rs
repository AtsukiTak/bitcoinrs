mod blockchain;
mod block;

pub use self::blockchain::BlockChain;
pub use self::block::{BlockData, BlockDataLike, FullBlockData};

use bitcoin::blockdata::block::BlockHeader;

#[derive(Debug)]
pub struct NotFoundPrevBlock(pub BlockHeader);
