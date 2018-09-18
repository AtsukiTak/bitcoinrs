use std::{io::Cursor, net::SocketAddr};
use bitcoin::network::{address::Address, constants::{Network, SERVICES, USER_AGENT}, encodable::ConsensusDecodable,
                       message::{CommandString, NetworkMessage, RawNetworkMessage},
                       serialize::{serialize, Error as BitcoinSerializeError, RawDecoder}};
use bitcoin::util::hash::Sha256dHash;

use futures::future::{result, Future};
use tokio::{io::{AsyncRead, ReadHalf, WriteHalf}, net::TcpStream};

use error::Error;


/*
 * Socket
 */
pub struct Socket
{
    send_socket: SendSocket,
    recv_socket: RecvSocket,
}

pub struct SendSocket
{
    socket: WriteHalf<TcpStream>,
    network: Network,
    user_agent: &'static str,
    local_addr: Address, // Change it into SocketAddr,
    remote_addr: Address,
}

pub struct RecvSocket
{
    socket: ReadHalf<TcpStream>,
    network: Network,
    user_agent: &'static str,
    local_addr: Address, // Change it into SocketAddr,
    remote_addr: Address,
}

impl Socket
{
    pub fn open(addr: &SocketAddr, network: Network) -> impl Future<Item = Socket, Error = Error>
    {
        TcpStream::connect(addr).map_err(Error::from).and_then(move |socket| {
            let local_addr = Address::new(&socket.local_addr().unwrap(), SERVICES);
            let remote_addr = Address::new(&socket.peer_addr().unwrap(), SERVICES);
            let (read, write) = socket.split();
            Ok(Socket {
                send_socket: SendSocket::new(write, network, local_addr.clone(), remote_addr.clone()),
                recv_socket: RecvSocket::new(read, network, local_addr, remote_addr),
            })
        })
    }

    pub fn remote_addr(&self) -> &Address
    {
        &self.send_socket.remote_addr()
    }

    pub fn local_addr(&self) -> &Address
    {
        &self.send_socket.local_addr()
    }

    pub fn user_agent(&self) -> &'static str
    {
        self.send_socket.user_agent()
    }

    pub fn send_msg(self, msg: NetworkMessage) -> impl Future<Item = Self, Error = Error>
    {
        let (s, r) = (self.send_socket, self.recv_socket);
        s.send_msg(msg).map(|s| {
            Socket {
                send_socket: s,
                recv_socket: r,
            }
        })
    }

    pub fn recv_msg(self) -> impl Future<Item = (NetworkMessage, Self), Error = Error>
    {
        let (s, r) = (self.send_socket, self.recv_socket);
        r.recv_msg().map(|(msg, r)| {
            let socket = Socket {
                send_socket: s,
                recv_socket: r,
            };
            (msg, socket)
        })
    }
}


impl ::std::fmt::Debug for Socket
{
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error>
    {
        write!(
            f,
            "Socket {{ remote: {:?}, local: {:?} }}",
            self.remote_addr(),
            self.local_addr()
        )
    }
}

impl ::std::fmt::Display for Socket
{
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error>
    {
        write!(f, "Socket to peer {:?}", self.remote_addr().address)
    }
}

/* Sending Half */

impl SendSocket
{
    fn new(socket: WriteHalf<TcpStream>, network: Network, local_addr: Address, remote_addr: Address) -> SendSocket
    {
        SendSocket {
            socket,
            network,
            local_addr,
            remote_addr,
            user_agent: USER_AGENT,
        }
    }

    pub fn local_addr(&self) -> &Address
    {
        &self.local_addr
    }

    pub fn remote_addr(&self) -> &Address
    {
        &self.remote_addr
    }

    pub fn user_agent(&self) -> &'static str
    {
        self.user_agent
    }

    pub fn send_msg(self, msg: NetworkMessage) -> impl Future<Item = Self, Error = Error>
    {
        debug!("Send a message {:?}", msg);
        let serialized = encode(msg, self.network);
        let (socket, network, l_addr, r_addr) = (self.socket, self.network, self.local_addr, self.remote_addr);

        ::tokio::io::write_all(socket, serialized)
            .and_then(|(socket, _)| ::tokio::io::flush(socket))
            .map_err(Error::from)
            .map(move |socket| SendSocket::new(socket, network, l_addr, r_addr))
    }
}

fn encode(msg: NetworkMessage, network: Network) -> Vec<u8>
{
    let msg = RawNetworkMessage {
        magic: network.magic(),
        payload: msg,
    };
    serialize(&msg).unwrap() // Never fail
}

impl ::std::fmt::Debug for SendSocket
{
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error>
    {
        write!(
            f,
            "SendSocket {{ remote: {:?}, local: {:?} }}",
            self.remote_addr(),
            self.local_addr()
        )
    }
}

impl ::std::fmt::Display for SendSocket
{
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error>
    {
        write!(f, "SendSocket to peer {:?}", self.remote_addr().address)
    }
}


/* Receiving Half */

impl RecvSocket
{
    fn new(socket: ReadHalf<TcpStream>, network: Network, local_addr: Address, remote_addr: Address) -> RecvSocket
    {
        RecvSocket {
            socket,
            network,
            local_addr,
            remote_addr,
            user_agent: USER_AGENT,
        }
    }

    pub fn local_addr(&self) -> &Address
    {
        &self.local_addr
    }

    pub fn remote_addr(&self) -> &Address
    {
        &self.remote_addr
    }

    pub fn user_agent(&self) -> &'static str
    {
        self.user_agent
    }

    pub fn recv_msg(self) -> impl Future<Item = (NetworkMessage, Self), Error = Error>
    {
        let (socket, network, l_addr, r_addr) = (self.socket, self.network, self.local_addr, self.remote_addr);
        let header_buf: [u8; RAW_NETWORK_MESSAGE_HEADER_SIZE] = [0; RAW_NETWORK_MESSAGE_HEADER_SIZE];
        ::tokio::io::read_exact(socket, header_buf)
            .map_err(Error::from)
            .and_then(move |(socket, bytes)| {
                let header = decode_msg_header(&bytes, &network)?;
                Ok((socket, header))
            })
            .and_then(|(socket, header)| {
                let mut buf = Vec::with_capacity(header.payload_size as usize);
                buf.resize(header.payload_size as usize, 0);
                ::tokio::io::read_exact(socket, buf)
                    .map_err(Error::from)
                    .map(|(socket, bytes)| (socket, bytes, header))
            })
            .and_then(move |(socket, bytes, header)| {
                let msg = decode_and_check_msg_payload(&bytes, &header, &network)?;
                let socket = RecvSocket::new(socket, network, l_addr, r_addr);
                Ok((msg, socket))
            })
    }
}

const RAW_NETWORK_MESSAGE_HEADER_SIZE: usize = 24;

struct RawNetworkMessageHeader
{
    command_name: CommandString,
    payload_size: u32,
    checksum: [u8; 4],
}

/// # Panic
/// If length of `src` is not 24 bytes.
fn decode_msg_header(src: &[u8], network: &Network) -> Result<RawNetworkMessageHeader, Error>
{
    assert!(src.len() == RAW_NETWORK_MESSAGE_HEADER_SIZE);

    debug!("Decode message header");

    let mut decoder = RawDecoder::new(Cursor::new(src));

    let magic = u32::consensus_decode(&mut decoder)?;
    if magic != network.magic() {
        return Err(Error::from(BitcoinSerializeError::UnexpectedNetworkMagic {
            expected: network.magic(),
            actual: magic,
        }));
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
    src: &[u8],
    header: &RawNetworkMessageHeader,
    network: &Network,
) -> Result<NetworkMessage, Error>
{
    assert!(src.len() as u32 == header.payload_size);

    let mut decoder = RawDecoder::new(Cursor::new(src));

    // Check a checksum
    let expected_checksum = sha2_checksum(&src);
    if expected_checksum != header.checksum {
        warn!("bad checksum");
        return Err(Error::from(BitcoinSerializeError::InvalidChecksum {
            expected: expected_checksum,
            actual: header.checksum,
        }));
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
            return Err(Error::from(BitcoinSerializeError::UnrecognizedNetworkCommand(
                cmd.into(),
            )));
        },
    };

    Ok(msg)
}

fn sha2_checksum(data: &[u8]) -> [u8; 4]
{
    let checksum = Sha256dHash::from_data(data);
    [checksum[0], checksum[1], checksum[2], checksum[3]]
}

impl ::std::fmt::Debug for RecvSocket
{
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error>
    {
        write!(
            f,
            "RecvSocket {{ remote: {:?}, local: {:?} }}",
            self.remote_addr(),
            self.local_addr()
        )
    }
}

impl ::std::fmt::Display for RecvSocket
{
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error>
    {
        write!(f, "RecvSocket to peer {:?}", self.remote_addr().address)
    }
}
