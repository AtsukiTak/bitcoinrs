use bitcoin::network::{address::Address, message::NetworkMessage,
                       message_blockdata::{GetBlocksMessage, GetHeadersMessage, Inventory}};
use bitcoin::blockdata::block::{Block, LoneBlockHeader};

use futures::{Future, Stream};
use tokio::{io::{AsyncRead, AsyncWrite, WriteHalf}, net::TcpStream};
use actix::{io::{FramedWrite, WriteHandler}, prelude::*};

use p2p::socket::{BtcEncoder, Socket};
use error::Error;

const DEFAULT_MAIL_BOX_SIZE: usize = 8;
const DEFAULT_ADDRS_CAPACITY: usize = 8;
const DEFAULT_INVS_CAPACITY: usize = 8;

#[derive(Message, Debug)]
pub struct P2PMessage(NetworkMessage);

#[derive(Message)]
pub struct GetBlocksRequest
{
    msg: GetBlocksMessage,
    addr: Recipient<BlockResponse>,
}

#[derive(Message, Debug)]
pub struct BlockResponse
{
    block: Block,
}

#[derive(Message)]
pub struct GetHeadersRequest
{
    msg: GetHeadersMessage,
    addr: Recipient<HeadersResponse>,
}

#[derive(Message, Debug)]
pub struct HeadersResponse
{
    headers: Vec<LoneBlockHeader>,
}

pub struct Connection
{
    framed_write: FramedWrite<WriteHalf<TcpStream>, BtcEncoder>,

    addrs: Vec<(u32, Address)>,
    invs: Vec<Inventory>,
}

/// Establish a new outgoing connection.
pub fn create_outgoing_connection(socket: Socket<TcpStream>, ctx: &mut Context<Connection>) -> Connection
{
    let (read_socket, write_socket) = socket.split();

    let msg_stream = read_socket.recv_msg_stream().map(|m| P2PMessage(m));
    ctx.add_stream(msg_stream);

    let framed_write = write_socket.into_framed_write(ctx);

    Connection::new(framed_write)
}

impl Actor for Connection
{
    type Context = Context<Self>;
}

impl Connection
{
    fn new(framed_write: FramedWrite<WriteHalf<TcpStream>, BtcEncoder>) -> Connection
    {
        Connection {
            framed_write,

            addrs: Vec::new(),
            invs: Vec::new(),
        }
    }

    fn send_p2p_msg(&mut self, msg: NetworkMessage, ctx: &mut Context<Self>)
    {
        self.framed_write.write(msg)
    }
}

impl WriteHandler<Error> for Connection
{
    fn error(&mut self, err: Error, ctx: &mut Self::Context) -> Running
    {
        info!("Error while sending msg : {:?}", err);
        Running::Stop
    }

    fn finished(&mut self, ctx: &mut Self::Context)
    {
        debug!("Finish to send msg");
    }
}


/* Handle P2P Message */

impl StreamHandler<P2PMessage, Error> for Connection
{
    fn handle(&mut self, msg: P2PMessage, ctx: &mut Self::Context)
    {
        use self::NetworkMessage::*;
        match msg.0 {
            Addr(addrs) => self.handle_addr_msg(addrs, ctx),
            // Inv(invs) => self.handle_invs_msg(invs, ctx),
            // Block(block) => self.handle_block_msg(block, ctx),
            // Headers(headers) => self.handle_headers_msg(headers, ctx),
            Ping(nonce) => self.handle_ping_msg(nonce, ctx),
            another => {
                info!("Receive unexpected network msg. {:?}", another);
            },
        }
    }

    fn error(&mut self, err: Error, ctx: &mut Self::Context) -> Running
    {
        info!("Catch error on socket : {:?}", err);
        Running::Stop
    }
}

impl Connection
{
    fn handle_addr_msg(&mut self, addrs: Vec<(u32, Address)>, ctx: &mut Context<Self>)
    {
        self.addrs.extend_from_slice(&addrs[..]);
        self.addrs.drain(..DEFAULT_ADDRS_CAPACITY);
    }

    /*
    fn handle_invs_msg(&mut self, invs: Vec<Inventory>, ctx: &mut Context<Self>)
    {
        if let Some(invs_tx) = self.waiting_invs.take() {
            invs_tx.send(invs); // Does not care if it fail.
        } else {
            self.invs.extend_from_slice(&invs[..]);
            self.invs.drain(..DEFAULT_INVS_CAPACITY);
        }
    }

    fn handle_block_msg(&mut self, block: Block, ctx: &mut Context<Self>)
    {
        if let Some(block_tx) = self.waiting_blocks.take() {
            block_tx.send(block); // Does not care if it fail.
        } else {
            warn!("Receive unexpected Block message");
            ctx.stop();
        }
    }

    fn handle_headers_msg(&mut self, headers: Vec<LoneBlockHeader>, ctx: &mut Context<Self>)
    {
        if let Some(headers_tx) = self.waiting_headers.take() {
            headers_tx.send(headers);
        } else {
            warn!("Receive unexpected Headers message");
            ctx.stop();
        }
    }
    */

    fn handle_ping_msg(&mut self, nonce: u64, ctx: &mut Context<Self>)
    {
        let pong = NetworkMessage::Pong(nonce);
        self.send_p2p_msg(pong, ctx);
    }
}


/* Handle requests */
