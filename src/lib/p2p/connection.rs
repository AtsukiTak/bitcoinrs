use std::time::Duration;

use bitcoin::network::{address::Address, message::NetworkMessage,
                       message_blockdata::{GetHeadersMessage, InvType, Inventory}};
use bitcoin::blockdata::block::{Block, LoneBlockHeader};
use bitcoin::util::hash::Sha256dHash;
use bitcoin::BitcoinHash;

use futures::{Future, Stream};
use tokio::{io::WriteHalf, net::TcpStream};
use actix::prelude::*;

use p2p::HandshakedSocket;
use error::Error;

const DEFAULT_ADDRS_CAPACITY: usize = 8;
const SEND_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Message, Debug)]
pub struct P2PMessage(NetworkMessage);

#[derive(Message)]
/// This message corresponds to `getdata` message in bitcoin protocol.
/// If peer does not have requested block data, peer does not respond anything.
/// Bitcoin core implementation is that it wait up to two seconds and then disconnect.
pub struct GetBlocksRequest
{
    pub block_hashes: Vec<Sha256dHash>,
    pub addr: Recipient<BlockResponse>,
}

#[derive(Message)]
/// A response message to GetBlocksRequest.
/// Sender probably receive same number of `BlockResponse` with request hashes..
/// But sometime that number is less (see `GetBlocksRequest` document).
/// Sender **SHOULD** set timeout.
/// 2 seconds are recommended.
pub struct BlockResponse(pub Block);

#[derive(Message)]
/// This message corresponds to `getheaders` message in bitcoin protocol.
pub struct GetHeadersRequest
{
    pub msg: GetHeadersMessage,
    pub addr: Recipient<HeadersResponse>,
}

#[derive(Message)]
/// This message corresponds to `headers` message in bitcoin protocol.
pub struct HeadersResponse(pub Vec<LoneBlockHeader>);

#[derive(Message)]
/// Start to subscribe incoming `inv` message.
/// Sender may receive a lot of `PublishInv` message.
pub struct SubscribeInv
{
    pub addr: Recipient<PublishInv>,
}

#[derive(Message)]
/// This message corresponds to `inv` message in bitcoin protocol.
pub struct PublishInv(pub Vec<Inventory>);

#[derive(Message)]
/// Force to gracefully shutdown connection.
pub struct Disconnect();

/// # Note
/// The behavior of `Connection` follows bitcoin protocol.
/// e.g. after GetBlocksRequest is sent, if connecting peer couldn't find requested block peer does
/// not response anything.
/// So `Connection` is also response nothing.
pub struct Connection
{
    // it should not be None except during waiting to complete sending
    write_socket: Option<HandshakedSocket<WriteHalf<TcpStream>>>,
    socket_stream_handle: SpawnHandle,

    waiting_blocks: Option<WaitingBlocks>,
    waiting_headers: Option<WaitingHeaders>,
    subscribe_invs: Option<Recipient<PublishInv>>,

    addrs: Vec<(u32, Address)>,
}

impl Actor for Connection
{
    type Context = Context<Self>;
}

impl Connection
{
    pub fn start_actor(socket: HandshakedSocket<TcpStream>) -> Addr<Self>
    {
        Connection::create(move |ctx| {
            let (read_socket, write_socket) = socket.split();

            let msg_stream = read_socket.recv_msg_stream().map(|m| P2PMessage(m));
            let socket_stream_handle = ctx.add_stream(msg_stream);

            Connection::new(write_socket, socket_stream_handle)
        })
    }

    fn new(write_socket: HandshakedSocket<WriteHalf<TcpStream>>, socket_stream_handle: SpawnHandle) -> Connection
    {
        Connection {
            write_socket: Some(write_socket),
            socket_stream_handle,

            waiting_blocks: None,
            waiting_headers: None,
            subscribe_invs: None,

            addrs: Vec::new(),
        }
    }

    fn send_p2p_msg(&mut self, msg: NetworkMessage, ctx: &mut Context<Self>)
    {
        let write_socket = self.write_socket.take().expect("BUG!!");
        let f = write_socket
            .send_msg(msg)
            .into_actor(self)
            .map(|socket, actor, _ctx| {
                actor.write_socket = Some(socket);
            })
            .map_err(|e, _actor, ctx| {
                info!("Socket is closed : {:?}", e);
                info!("Close connection as well");
                ctx.stop();
            });
        ctx.wait(f);
    }
}

impl Handler<Disconnect> for Connection
{
    type Result = ();

    fn handle(&mut self, _msg: Disconnect, ctx: &mut Self::Context)
    {
        let _ = self.write_socket.take().unwrap().shutdown().wait();
        ctx.cancel_future(self.socket_stream_handle);
        ctx.stop();
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
            Inv(invs) => self.handle_invs_msg(invs, ctx),
            Block(block) => self.handle_block_msg(block, ctx),
            Headers(headers) => self.handle_headers_msg(headers, ctx),
            Ping(nonce) => self.handle_ping_msg(nonce, ctx),
            another => {
                info!("Receive unexpected network msg. {:?}", another);
            },
        }
    }

    fn error(&mut self, err: Error, _ctx: &mut Self::Context) -> Running
    {
        info!("Catch error on socket : {:?}", err);
        Running::Stop
    }
}

struct WaitingBlocks
{
    addr: Recipient<BlockResponse>,
    block_hashes: Vec<Sha256dHash>,
}

struct WaitingHeaders
{
    addr: Recipient<HeadersResponse>,
}

impl Connection
{
    fn stop_misbehaving_connection(&mut self, ctx: &mut Context<Self>)
    {
        info!("Peer misbehaves. Close connection");
        ctx.stop();
    }

    fn handle_addr_msg(&mut self, addrs: Vec<(u32, Address)>, _ctx: &mut Context<Self>)
    {
        self.addrs.extend_from_slice(&addrs[..]);
        self.addrs.drain(..DEFAULT_ADDRS_CAPACITY);
    }

    fn handle_block_msg(&mut self, block: Block, ctx: &mut Context<Connection>)
    {
        let maybe_waiting_blocks = self.waiting_blocks.take();
        match maybe_waiting_blocks {
            None => {
                self.stop_misbehaving_connection(ctx);
            },
            Some(mut waiting) => {
                let block_hash = block.bitcoin_hash();
                let maybe_idx = waiting.block_hashes.iter().position(|h| *h == block_hash);
                match maybe_idx {
                    None => {
                        self.stop_misbehaving_connection(ctx);
                        return;
                    },
                    Some(idx) => waiting.block_hashes.remove(idx),
                };
                let send_f = waiting.addr.send(BlockResponse(block)).timeout(SEND_TIMEOUT);
                let f = send_f
                    .map(move |()| waiting)
                    .into_actor(self)
                    .map(|waiting, actor, _ctx| {
                        if !waiting.block_hashes.is_empty() {
                            actor.waiting_blocks = Some(waiting);
                        }
                    })
                    .map_err(|e, _actor, _ctx| {
                        debug!("Fail to send msg : {:?}", e);
                    });
                ctx.wait(f);
            },
        }
    }

    fn handle_invs_msg(&mut self, invs: Vec<Inventory>, ctx: &mut Context<Self>)
    {
        let maybe_subscriber = self.subscribe_invs.take();
        if let Some(subscriber) = maybe_subscriber {
            let send_f = subscriber.send(PublishInv(invs)).timeout(SEND_TIMEOUT);
            let f = send_f
                .map(move |_| subscriber)
                .into_actor(self)
                .map(|subscriber, actor, _ctx| {
                    actor.subscribe_invs = Some(subscriber);
                })
                .map_err(|e, _actor, _ctx| {
                    debug!("Fail to send msg : {:?}", e);
                });
            ctx.wait(f);
        } else {
            debug!("Peer sends Inv message but no subscriber is set, so discard it.");
        }
    }

    fn handle_headers_msg(&mut self, headers: Vec<LoneBlockHeader>, ctx: &mut Context<Self>)
    {
        let maybe_waiting_headers = self.waiting_headers.take();
        match maybe_waiting_headers {
            None => {
                info!("We don't wait headers but received.");
                self.stop_misbehaving_connection(ctx);
            },
            Some(waiting_headers) => {
                let f = waiting_headers
                    .addr
                    .send(HeadersResponse(headers))
                    .map_err(|_e| ())
                    .into_actor(self);
                ctx.wait(f);
            },
        }
    }

    fn handle_ping_msg(&mut self, nonce: u64, ctx: &mut Context<Self>)
    {
        let pong = NetworkMessage::Pong(nonce);
        self.send_p2p_msg(pong, ctx);
    }
}

/* Handle GetBlocksRequest */

impl Handler<GetBlocksRequest> for Connection
{
    type Result = ();

    fn handle(&mut self, req: GetBlocksRequest, ctx: &mut Context<Connection>)
    {
        if self.waiting_blocks.is_some() {
            info!("Can not request GetBlockRequest in parallel. A new request is dropped.");
            return;
        }

        // Send Inv message to peer
        let invs: Vec<_> = req.block_hashes
            .iter()
            .map(|hash| {
                Inventory {
                    inv_type: InvType::Block,
                    hash: *hash,
                }
            })
            .collect();
        let msg = NetworkMessage::GetData(invs);
        self.send_p2p_msg(msg, ctx);

        let waiting_blocks = WaitingBlocks {
            addr: req.addr,
            block_hashes: req.block_hashes,
        };
        self.waiting_blocks = Some(waiting_blocks);
    }
}

/* Handle GetHeadersRequest */

impl Handler<GetHeadersRequest> for Connection
{
    type Result = ();

    fn handle(&mut self, req: GetHeadersRequest, ctx: &mut Context<Self>)
    {
        if self.waiting_headers.is_some() {
            info!("Can not request GetHeadersRequest in parallel. A new request is dropped.");
            return;
        }

        // Send GetHeaders message to peer
        let msg = NetworkMessage::GetHeaders(req.msg);
        self.send_p2p_msg(msg, ctx);

        let waiting_headers = WaitingHeaders { addr: req.addr };
        self.waiting_headers = Some(waiting_headers);
    }
}
