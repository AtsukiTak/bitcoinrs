use std::{io::Cursor, time::{SystemTime, UNIX_EPOCH}};
use bitcoin::network::{address::Address, constants::{Network, PROTOCOL_VERSION}, encodable::ConsensusDecodable,
                       message::{CommandString, NetworkMessage, RawNetworkMessage}, message_network::VersionMessage,
                       serialize::{serialize, Error as BitcoinSerializeError, RawDecoder}};
use bitcoin::util::hash::Sha256dHash;

use futures::{Future, IntoFuture, Sink, Stream};
use tokio::{codec::{Encoder, FramedWrite}, io::{shutdown, AsyncRead, AsyncWrite, ReadHalf, Shutdown, WriteHalf},
            net::TcpStream};
use bytes::BytesMut;

use error::{Error, ErrorKind};

pub const USER_AGENT: &str = "bitcoinrs v0.0";

#[derive(Debug)]
pub struct Socket<S>
{
    socket: S,
    network: Network,
}

#[derive(Debug)]
pub struct HandshakedSocket<S>(Socket<S>);

impl<S> Socket<S>
{
    pub fn new(socket: S, network: Network) -> Socket<S>
    {
        Socket { socket, network }
    }

    fn breakdown(self) -> (S, Network)
    {
        (self.socket, self.network)
    }

    pub fn split(self) -> (Socket<ReadHalf<S>>, Socket<WriteHalf<S>>)
    where S: AsyncRead + AsyncWrite
    {
        let (socket, net) = self.breakdown();
        let (r, w) = socket.split();
        (Socket::new(r, net.clone()), Socket::new(w, net))
    }

    pub fn shutdown(self) -> Shutdown<S>
    where S: AsyncWrite
    {
        shutdown(self.socket)
    }

    pub fn send_msg(self, msg: NetworkMessage) -> impl Future<Item = Self, Error = Error>
    where S: AsyncWrite
    {
        debug!("Send a message {:?}", msg);
        let (socket, network) = self.breakdown();
        let serialized = encode(msg, network.clone());

        ::tokio::io::write_all(socket, serialized)
            .and_then(|(socket, _)| ::tokio::io::flush(socket))
            .map_err(Error::from)
            .map(move |socket| Socket::new(socket, network))
    }

    pub fn send_msg_sink(self) -> impl Sink<SinkItem = NetworkMessage, SinkError = Error>
    where S: AsyncWrite
    {
        let (socket, network) = self.breakdown();
        let encoder = BtcEncoder { network };
        FramedWrite::new(socket, encoder)
    }

    pub fn recv_msg(self) -> impl Future<Item = (NetworkMessage, Self), Error = Error>
    where S: AsyncRead
    {
        let (socket, network) = self.breakdown();
        let network2 = network.clone();
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
                let msg = decode_and_check_msg_payload(&bytes, &header)?;
                Ok((msg, Socket::new(socket, network2)))
            })
    }

    pub fn recv_msg_stream(self) -> impl Stream<Item = NetworkMessage, Error = Error>
    where S: AsyncRead
    {
        ::futures::stream::unfold(self, |s| Some(s.recv_msg()))
    }
}

impl Socket<TcpStream>
{
    pub fn begin_handshake(
        self,
        start_height: i32,
        services: u64,
        relay: bool,
    ) -> impl Future<Item = HandshakedSocket<TcpStream>, Error = Error>
    {
        begin_handshake(self, start_height, services, relay)
    }
}

impl<S> HandshakedSocket<S>
{
    pub fn split(self) -> (HandshakedSocket<ReadHalf<S>>, HandshakedSocket<WriteHalf<S>>)
    where S: AsyncRead + AsyncWrite
    {
        let (r, w) = self.0.split();
        (HandshakedSocket(r), HandshakedSocket(w))
    }

    pub fn shutdown(self) -> Shutdown<S>
    where S: AsyncWrite
    {
        self.0.shutdown()
    }

    pub fn send_msg(self, msg: NetworkMessage) -> impl Future<Item = Self, Error = Error>
    where S: AsyncWrite
    {
        self.0.send_msg(msg).map(|s| HandshakedSocket(s))
    }

    pub fn send_msg_sink(self) -> impl Sink<SinkItem = NetworkMessage, SinkError = Error>
    where S: AsyncWrite
    {
        self.0.send_msg_sink()
    }

    pub fn recv_msg(self) -> impl Future<Item = (NetworkMessage, Self), Error = Error>
    where S: AsyncRead
    {
        self.0.recv_msg().map(|(msg, socket)| (msg, HandshakedSocket(socket)))
    }

    pub fn recv_msg_stream(self) -> impl Stream<Item = NetworkMessage, Error = Error>
    where S: AsyncRead
    {
        self.0.recv_msg_stream()
    }
}

pub fn begin_handshake(
    socket: Socket<TcpStream>,
    start_height: i32,
    services: u64,
    relay: bool,
) -> impl Future<Item = HandshakedSocket<TcpStream>, Error = Error>
{
    version_msg(&socket.socket, start_height, services, relay)
        .into_future()
        .and_then(|v| socket.send_msg(NetworkMessage::Version(v)))
        .and_then(|socket| socket.recv_msg())
        .and_then(|(msg, socket)| {
            match msg {
                NetworkMessage::Version(v) => Ok((v, socket)),
                msg => {
                    info!("Fail to handshake. Expect Version msg but found {:?}", msg);
                    Err(Error::from(ErrorKind::MisbehavePeer))
                },
            }
        })
        .and_then(|(remote_v, socket)| check_remote_version_msg(remote_v).map(|()| socket))
        .and_then(|socket| socket.send_msg(NetworkMessage::Verack))
        .and_then(|socket| socket.recv_msg())
        .and_then(|(msg, socket)| {
            match msg {
                NetworkMessage::Verack => Ok(HandshakedSocket(socket)),
                msg => {
                    info!("Fail to handshake. Expect Verack msg but found {:?}", msg);
                    Err(Error::from(ErrorKind::MisbehavePeer))
                },
            }
        })
}

fn version_msg(socket: &TcpStream, start_height: i32, services: u64, relay: bool) -> Result<VersionMessage, Error>
{
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
    let sender = Address::new(&socket.local_addr()?, services.clone());
    let receiver = Address::new(&socket.peer_addr()?, services.clone());
    Ok(VersionMessage {
        version: PROTOCOL_VERSION,
        services,
        timestamp: ts,
        receiver,
        sender,
        nonce: 0,
        user_agent: USER_AGENT.into(),
        start_height,
        relay,
    })
}

fn check_remote_version_msg(_version: VersionMessage) -> Result<(), Error>
{
    // Currently does not check anything
    Ok(())
}

fn encode(msg: NetworkMessage, network: Network) -> Vec<u8>
{
    let msg = RawNetworkMessage {
        magic: network.magic(),
        payload: msg,
    };
    serialize(&msg).unwrap() // Never fail
}

struct BtcEncoder
{
    pub network: Network,
}

impl Encoder for BtcEncoder
{
    type Item = NetworkMessage;
    type Error = Error;
    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error>
    {
        let encoded = encode(item, self.network.clone());
        dst.extend_from_slice(encoded.as_slice());
        Ok(())
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
fn decode_and_check_msg_payload(src: &[u8], header: &RawNetworkMessageHeader) -> Result<NetworkMessage, Error>
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
