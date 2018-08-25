mod blockchainmut;
// mod blockchain;
mod blocktree;
mod block;

pub use self::blockchainmut::BlockChainMut;
// pub use self::blockchain::BlockChain;
pub use self::blocktree::BlockTree;
pub use self::block::{BlockData, BlockGenerator, DefaultBlockGenerator, FullBlockData, HeaderOnlyBlockData,
                      RawBlockData};

use bitcoin::blockdata::block::Block;

#[derive(Debug)]
pub struct NotFoundPrevBlock(pub Block);
