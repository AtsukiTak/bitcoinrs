use std::net::SocketAddr;
use bitcoin::network::{address::Address, constants::Network, message::NetworkMessage, socket::Socket};

use error::Error;

pub struct SyncSocket
{
    socket: Socket,
    remote_addr: Address,
    local_addr: Address,
}

impl SyncSocket
{
    pub fn open(addr: &SocketAddr, network: Network) -> Result<SyncSocket, Error>
    {
        let mut socket = Socket::new(network);
        socket.connect(format!("{}", addr.ip()).as_str(), addr.port())?;
        let remote_addr = socket.receiver_address()?;
        let local_addr = socket.sender_address()?;
        Ok(SyncSocket {
            socket,
            remote_addr,
            local_addr,
        })
    }

    pub fn remote_addr(&self) -> &Address
    {
        &self.remote_addr
    }

    pub fn local_addr(&self) -> &Address
    {
        &self.local_addr
    }

    pub fn user_agent(&self) -> &str
    {
        self.socket.user_agent.as_str()
    }

    pub fn send_msg(&mut self, msg: NetworkMessage) -> Result<(), Error>
    {
        info!("Send new msg to {:?} : {:?}", self.remote_addr, msg);
        Ok(self.socket.send_message(msg)?)
    }

    pub fn recv_msg(&mut self) -> Result<NetworkMessage, Error>
    {
        let msg = self.socket.receive_message()?;
        debug!("Receive a new msg from {:?} : {:?}", self.remote_addr, msg);
        Ok(msg)
    }
}

impl ::std::fmt::Debug for SyncSocket
{
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error>
    {
        write!(
            f,
            "SyncSocket {{ remote: {:?}, local: {:?} }}",
            self.remote_addr, self.local_addr
        )
    }
}

impl ::std::fmt::Display for SyncSocket
{
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        write!(f, "SyncSocket to peer {:?}", self.remote_addr.address)
    }
}

/*
struct BitcoinNetworkCodec {
    magic: u32,
}

impl BitcoinNetworkCodec {
    fn new(network: Network) -> BitcoinNetworkCodec {
        BitcoinNetworkCodec {
            magic: magic(network),
        }
    }

    fn decode_inner(&self, src: &[u8]) -> Result<Option<(NetworkMessage, usize)>, Error> {
        let (res, consumed) = {
            let mut decoder = RawDecoder::new(Cursor::new(src));
            let res = RawNetworkMessage::consensus_decode(&mut decoder);
            let cursor = decoder.into_inner();
            let consumed = cursor.position();
            (res, consumed)
        };
        match res {
            Ok(msg) => if msg.magic == self.magic {
                Ok(Some((msg.payload, consumed as usize)))
            } else {
                Err(Error::from(BitcoinError::BadNetworkMagic(
                    self.magic,
                    msg.magic,
                )))
            },
            Err(BitcoinError::ByteOrder(_err)) => Ok(None),
            Err(err) => Err(Error::from(err)),
        }
    }

    fn encode_inner(&self, item: NetworkMessage) -> Result<Vec<u8>, Error> {
        let msg = RawNetworkMessage {
            magic: self.magic,
            payload: item,
        };
        Ok(serialize(&msg)?)
    }
}

impl Decoder for BitcoinNetworkCodec {
    type Item = NetworkMessage;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self.decode_inner(&src) {
            Ok(Some((msg, consumed))) => {
                src.split_to(consumed);
                Ok(Some(msg))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

impl Encoder for BitcoinNetworkCodec {
    type Item = NetworkMessage;
    type Error = Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let vec = self.encode_inner(item)?;
        dst.extend_from_slice(&vec);
        Ok(())
    }
}
*/
