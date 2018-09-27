mod socket;
mod connection;

pub use self::socket::Socket;
pub use self::connection::Connection;

pub mod msg
{
    pub use super::connection::{BlockResponse, Disconnect, GetBlocksRequest, GetHeadersRequest, HeadersResponse,
                               PublishInv, SubscribeInv};
}
