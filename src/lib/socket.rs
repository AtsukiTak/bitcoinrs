use std::{io::Cursor, net::SocketAddr};
use bitcoin::network::{address::Address, constants::{magic, Network, SERVICES, USER_AGENT},
                       encodable::ConsensusDecodable, message::{CommandString, NetworkMessage, RawNetworkMessage},
                       serialize::{serialize, RawDecoder}, socket::Socket};
use bitcoin::util::Error as BitcoinError;
use bitcoin::util::hash::Sha256dHash;

use futures::Future;
use tokio_tcp::TcpStream as AsyncTcpStream;

use error::Error;


/*
 * AsyncSocket
 */
pub struct AsyncSocket
{
    socket: AsyncTcpStream,
    codec: BitcoinNetworkCodec,
    local_addr: Address,
    remote_addr: Address,
    user_agent: &'static str,
}

impl AsyncSocket
{
    pub fn open(addr: &SocketAddr, network: Network) -> impl Future<Item = AsyncSocket, Error = Error>
    {
        AsyncTcpStream::connect(addr)
            .map_err(Error::from)
            .and_then(move |socket| {
                debug!("Recv buffer size is {}", socket.recv_buffer_size().unwrap());
                let local_addr = Address::new(&socket.local_addr()?, SERVICES);
                let remote_addr = Address::new(&socket.peer_addr()?, SERVICES);
                Ok(AsyncSocket {
                    socket,
                    codec: BitcoinNetworkCodec::new(network),
                    local_addr,
                    remote_addr,
                    user_agent: USER_AGENT,
                })
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

    pub fn user_agent(&self) -> &'static str
    {
        self.user_agent
    }

    pub fn send_msg(self, msg: NetworkMessage) -> impl Future<Item = Self, Error = Error>
    {
        debug!("Send a message {:?}", msg);
        let serialized = self.codec.encode_inner(msg);
        let (socket, codec, local_addr, remote_addr) = (self.socket, self.codec, self.local_addr, self.remote_addr);

        ::tokio_io::io::write_all(socket, serialized)
            .and_then(|(socket, _)| ::tokio_io::io::flush(socket))
            .map_err(Error::from)
            .map(move |socket| {
                AsyncSocket {
                    socket,
                    codec,
                    local_addr,
                    remote_addr,
                    user_agent: USER_AGENT,
                }
            })
    }

    pub fn recv_msg(self) -> impl Future<Item = (NetworkMessage, Self), Error = Error>
    {
        let (socket, codec, local_addr, remote_addr) = (self.socket, self.codec, self.local_addr, self.remote_addr);
        let codec2 = codec.clone();
        let header_buf: [u8; RAW_NETWORK_MESSAGE_HEADER_SIZE] = [0; RAW_NETWORK_MESSAGE_HEADER_SIZE];
        ::tokio_io::io::read_exact(socket, header_buf)
            .map_err(Error::from)
            .and_then(move |(socket, bytes)| {
                let header = codec.decode_msg_header(&bytes)?;
                Ok((socket, header))
            })
            .and_then(|(socket, header)| {
                let mut buf = Vec::with_capacity(header.payload_size as usize);
                buf.resize(header.payload_size as usize, 0);
                ::tokio_io::io::read_exact(socket, buf)
                    .map_err(Error::from)
                    .map(|(socket, bytes)| (socket, bytes, header))
            })
            .and_then(move |(socket, bytes, header)| {
                let msg = codec2.decode_and_check_msg_payload(&bytes, &header)?;
                let socket = AsyncSocket {
                    socket,
                    codec: codec2,
                    local_addr,
                    remote_addr,
                    user_agent: USER_AGENT,
                };
                Ok((msg, socket))
            })
    }
}

impl ::std::fmt::Debug for AsyncSocket
{
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error>
    {
        write!(
            f,
            "AsyncSocket {{ remote: {:?}, local: {:?} }}",
            self.remote_addr, self.local_addr
        )
    }
}

impl ::std::fmt::Display for AsyncSocket
{
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error>
    {
        write!(f, "AsyncSocket to peer {:?}", self.remote_addr.address)
    }
}


#[derive(Copy, Clone)]
struct BitcoinNetworkCodec
{
    magic: u32,
}

const RAW_NETWORK_MESSAGE_HEADER_SIZE: usize = 24;

struct RawNetworkMessageHeader
{
    command_name: CommandString,
    payload_size: u32,
    checksum: [u8; 4],
}

impl BitcoinNetworkCodec
{
    fn new(network: Network) -> BitcoinNetworkCodec
    {
        BitcoinNetworkCodec { magic: magic(network) }
    }

    /// # Panic
    /// If length of `src` is not 24 bytes.
    fn decode_msg_header(&self, src: &[u8]) -> Result<RawNetworkMessageHeader, Error>
    {
        assert!(src.len() == RAW_NETWORK_MESSAGE_HEADER_SIZE);

        debug!("Decode message header");

        let mut decoder = RawDecoder::new(Cursor::new(src));

        let magic = u32::consensus_decode(&mut decoder)?;
        if magic != self.magic {
            return Err(Error::from(BitcoinError::BadNetworkMagic(self.magic, magic)));
        }

        let command_name = CommandString::consensus_decode(&mut decoder)?;
        let payload_size = u32::consensus_decode(&mut decoder)?;
        let checksum = <[u8; 4]>::consensus_decode(&mut decoder)?;

        Ok(RawNetworkMessageHeader {
            command_name,
            payload_size,
            checksum,
        })
    }

    /// # Panic
    /// If length of `src` is not `header.payload_size`.
    fn decode_and_check_msg_payload(
        &self,
        src: &[u8],
        header: &RawNetworkMessageHeader,
    ) -> Result<NetworkMessage, Error>
    {
        assert!(src.len() as u32 == header.payload_size);

        let mut decoder = RawDecoder::new(Cursor::new(src));

        // Check a checksum
        let expected_checksum = sha2_checksum(&src);
        if expected_checksum != header.checksum {
            warn!("bad checksum");
            return Err(Error::from(BitcoinError::ParseFailed));
        }

        let msg = match &header.command_name.0[..] {
            "version" => NetworkMessage::Version(ConsensusDecodable::consensus_decode(&mut decoder)?),
            "verack" => NetworkMessage::Verack,
            "addr" => NetworkMessage::Addr(ConsensusDecodable::consensus_decode(&mut decoder)?),
            "inv" => NetworkMessage::Inv(ConsensusDecodable::consensus_decode(&mut decoder)?),
            "getdata" => NetworkMessage::GetData(ConsensusDecodable::consensus_decode(&mut decoder)?),
            "notfound" => NetworkMessage::NotFound(ConsensusDecodable::consensus_decode(&mut decoder)?),
            "getblocks" => NetworkMessage::GetBlocks(ConsensusDecodable::consensus_decode(&mut decoder)?),
            "getheaders" => NetworkMessage::GetHeaders(ConsensusDecodable::consensus_decode(&mut decoder)?),
            "mempool" => NetworkMessage::MemPool,
            "block" => NetworkMessage::Block(ConsensusDecodable::consensus_decode(&mut decoder)?),
            "headers" => NetworkMessage::Headers(ConsensusDecodable::consensus_decode(&mut decoder)?),
            "getaddr" => NetworkMessage::GetAddr,
            "ping" => NetworkMessage::Ping(ConsensusDecodable::consensus_decode(&mut decoder)?),
            "pong" => NetworkMessage::Pong(ConsensusDecodable::consensus_decode(&mut decoder)?),
            "tx" => NetworkMessage::Tx(ConsensusDecodable::consensus_decode(&mut decoder)?),
            "alert" => NetworkMessage::Alert(ConsensusDecodable::consensus_decode(&mut decoder)?),
            cmd => {
                warn!("unrecognized network command : {}", cmd);
                return Err(Error::from(BitcoinError::ParseFailed));
            },
        };

        Ok(msg)
    }

    fn encode_inner(&self, item: NetworkMessage) -> Vec<u8>
    {
        let msg = RawNetworkMessage {
            magic: self.magic,
            payload: item,
        };
        serialize(&msg).unwrap() // Never fail
    }
}

fn sha2_checksum(data: &[u8]) -> [u8; 4]
{
    let checksum = Sha256dHash::from_data(data);
    [checksum[0], checksum[1], checksum[2], checksum[3]]
}
