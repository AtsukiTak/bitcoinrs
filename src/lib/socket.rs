use std::{io::Cursor, net::SocketAddr};
use bitcoin::network::{constants::{magic, Network}, encodable::ConsensusDecodable,
                       message::{NetworkMessage, RawNetworkMessage},
                       serialize::{serialize, RawDecoder}};
use bitcoin::util::Error as BitcoinError;
use futures::{Future, Sink, Stream};
use tokio::net::TcpStream;
use tokio_codec::{Decoder, Encoder, Framed};
use bytes::BytesMut;

use error::Error;

pub struct SyncSocket {
    // Sometime, socket is needed to be taken temporary. e.g. send_msg
    socket: Option<Framed<TcpStream, BitcoinNetworkCodec>>,

    remote_addr: SocketAddr,
    local_addr: SocketAddr,
}

impl SyncSocket {
    pub fn establish(addr: &SocketAddr, network: Network) -> Result<SyncSocket, Error> {
        let socket = TcpStream::connect(addr).wait()?;
        let local_addr = socket.local_addr()?;
        let codec = BitcoinNetworkCodec::new(network);
        Ok(SyncSocket {
            socket: Some(codec.framed(socket)),
            remote_addr: addr.clone(),
            local_addr: local_addr,
        })
    }

    pub fn send_msg(&mut self, msg: NetworkMessage) -> Result<(), Error> {
        info!("Send new msg to {} : {:?}", self.remote_addr, msg);
        let socket = self.socket.take().unwrap();
        let socket = socket.send(msg).wait()?;
        self.socket = Some(socket);
        Ok(())
    }
}

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
