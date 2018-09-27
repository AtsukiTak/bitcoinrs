mod socket;
mod connection;

pub use self::socket::Socket;
pub use self::connection::Connection;

pub mod msg
{
    pub use self::connection::{BlockResponse, Disconnect, GetBlockRequest, GetHeadersRequest, HeadersResponse,
                               PublishInv, SubscribeInv};
}
