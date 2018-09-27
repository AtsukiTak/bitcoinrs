use std::io::Cursor;
use bitcoin::network::{constants::Network, encodable::ConsensusDecodable,
                       message::{CommandString, NetworkMessage, RawNetworkMessage},
                       serialize::{serialize, Error as BitcoinSerializeError, RawDecoder}};
use bitcoin::util::hash::Sha256dHash;

use futures::{Future, Sink, Stream};
use tokio::{codec::{Encoder, FramedWrite}, io::{shutdown, AsyncRead, AsyncWrite, ReadHalf, Shutdown, WriteHalf}};
use bytes::BytesMut;

use error::Error;

pub struct Socket<S>
{
    socket: S,
    network: Network,
}

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
