use actix::prelude::*;
use futures::Future;

use blockchain::BlockChain;
use p2p::{Connection, msg::{Disconnect, GetHeadersRequest, HeadersResponse}};

const NUM_MAX_HEADERS_IN_MSG: usize = 2000;

pub struct SyncBlockChain
{
    // This should not be None unless all process is completed
    blockchain: Option<BlockChain>,
    connection: Addr<Connection>,
    notify: Recipient<SyncBlockChainResult>,
}

#[derive(Message)]
pub enum SyncBlockChainResult
{
    Complete(BlockChain),
    Error(BlockChain),
}

impl SyncBlockChain
{
    pub fn new(
        blockchain: BlockChain,
        conn: Addr<Connection>,
        notify: Recipient<SyncBlockChainResult>,
    ) -> SyncBlockChain
    {
        SyncBlockChain {
            blockchain: Some(blockchain),
            connection: conn,
            notify,
        }
    }

    pub fn start_actor(
        blockchain: BlockChain,
        conn: Addr<Connection>,
        notify: Recipient<SyncBlockChainResult>,
    ) -> Addr<SyncBlockChain>
    {
        SyncBlockChain::new(blockchain, conn, notify).start()
    }

    fn blockchain(&self) -> &BlockChain
    {
        self.blockchain.as_ref().unwrap()
    }

    fn blockchain_mut(&mut self) -> &mut BlockChain
    {
        self.blockchain.as_mut().unwrap()
    }

    fn request_getheaders(&mut self, ctx: &mut Context<Self>)
    {
        let locator_hashes = self.blockchain().active_chain().locator_hashes_vec();
        let addr = ctx.address().recipient();
        let req = GetHeadersRequest { locator_hashes, addr };

        let f = self.connection
            .send(req)
            .map_err(|_e| debug!("Connection is already dropped"))
            .into_actor(self);
        // Stop task processing until successfully send a request
        ctx.wait(f);
    }

    /// Send error message and then stop actor.
    fn notify_err(&mut self, ctx: &mut Context<Self>)
    {
        let res = SyncBlockChainResult::Error(self.blockchain.take().unwrap());
        let f = self.notify
            .send(res)
            .map_err(|_e| debug!("Caller already dropped"))
            .into_actor(self)
            .map(|(), _actor, ctx| ctx.stop());
        ctx.wait(f);
    }

    /// Send complete message and then stop actor.
    fn notify_complete(&mut self, ctx: &mut Context<Self>)
    {
        let res = SyncBlockChainResult::Complete(self.blockchain.take().unwrap());
        let f = self.notify
            .send(res)
            .map_err(|_e| debug!("Caller already dropped"))
            .into_actor(self)
            .map(|(), _actor, ctx| ctx.stop());
        ctx.wait(f);
    }
}

impl Actor for SyncBlockChain
{
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context)
    {
        self.request_getheaders(ctx)
    }
}

impl Handler<HeadersResponse> for SyncBlockChain
{
    type Result = ();
    fn handle(&mut self, msg: HeadersResponse, ctx: &mut Context<Self>)
    {
        let is_finish = msg.0.len() == NUM_MAX_HEADERS_IN_MSG;
        for lone_header in msg.0 {
            if let Err(_e) = self.blockchain_mut().try_add(lone_header.header) {
                info!("Peer sends invalid block header. Disconnect");
                self.connection.do_send(Disconnect());
                return self.notify_err(ctx);
            }
        }
        if is_finish {
            self.notify_complete(ctx);
        } else {
            self.request_getheaders(ctx);
        }
    }
}
