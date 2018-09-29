mod socket;
mod connection;
mod sync_blockchain;

pub use self::socket::{begin_handshake, HandshakedSocket, Socket};
pub use self::connection::Connection;
pub use self::sync_blockchain::{SyncBlockChain, SyncBlockChainResult};

pub mod msg
{
    pub use super::connection::{BlockResponse, Disconnect, GetBlocksRequest, GetHeadersRequest, HeadersResponse,
                                PublishInv, SubscribeInv};
}
