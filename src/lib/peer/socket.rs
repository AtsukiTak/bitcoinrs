use std::net::SocketAddr;
use bitcoin::network::{address::Address, constants::{Network, USER_AGENT}, message::NetworkMessage,
                       socket::Socket as BitcoinSocket};

use futures::future::{result, Future};

use error::Error;


/*
 * Socket
 */
pub struct Socket
{
    socket: BitcoinSocket,
    local_addr: Address,
    remote_addr: Address,
    user_agent: &'static str,
}

impl Socket
{
    pub fn open(addr: &SocketAddr, network: Network) -> impl Future<Item = Socket, Error = Error>
    {
        fn inner(addr: &SocketAddr, network: Network) -> Result<Socket, Error>
        {
            let mut socket = BitcoinSocket::new(network);
            socket.connect(format!("{}", addr.ip()).as_str(), addr.port())?;

            let local_addr = socket.sender_address()?;
            let remote_addr = socket.receiver_address()?;
            Ok(Socket {
                socket,
                local_addr,
                remote_addr,
                user_agent: USER_AGENT,
            })
        }
        result(inner(addr, network))
    }

    pub fn remote_addr(&self) -> &Address
    {
        &self.remote_addr
    }

    pub fn local_addr(&self) -> &Address
    {
        &self.local_addr
    }

    pub fn user_agent(&self) -> &'static str
    {
        self.user_agent
    }

    pub fn send_msg(mut self, msg: NetworkMessage) -> impl Future<Item = Self, Error = Error>
    {
        debug!("Send a message {:?}", msg);
        let send_res = self.socket.send_message(msg);
        let res = match send_res {
            Ok(()) => Ok(self),
            Err(e) => Err(Error::from(e)),
        };
        result(res)
    }

    pub fn recv_msg(mut self) -> impl Future<Item = (NetworkMessage, Self), Error = Error>
    {
        let recv_res = self.socket.receive_message();
        let res = match recv_res {
            Ok(msg) => {
                debug!("Receive a new message {:?}", msg);
                Ok((msg, self))
            },
            Err(e) => Err(Error::from(e)),
        };
        result(res)
    }
}

impl ::std::fmt::Debug for Socket
{
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error>
    {
        write!(
            f,
            "Socket {{ remote: {:?}, local: {:?} }}",
            self.remote_addr, self.local_addr
        )
    }
}

impl ::std::fmt::Display for Socket
{
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error>
    {
        write!(f, "Socket to peer {:?}", self.remote_addr.address)
    }
}
