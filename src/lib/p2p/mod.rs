mod socket;
mod connection;

pub use self::socket::{HandshakedSocket, Socket, begin_handshake};
pub use self::connection::Connection;

pub mod msg
{
    pub use super::connection::{BlockResponse, Disconnect, GetBlocksRequest, GetHeadersRequest, HeadersResponse,
                                PublishInv, SubscribeInv};
}
