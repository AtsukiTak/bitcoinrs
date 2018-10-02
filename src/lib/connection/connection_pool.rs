use std::{collections::HashSet, net::{IpAddr, SocketAddr}, sync::{Arc, Mutex}, time::Duration};
use actix::prelude::*;
use trust_dns_resolver::{ResolverFuture, config::{ResolverConfig, ResolverOpts}, error::ResolveError};
use futures::Future;
use bitcoin::network::constants::Network;

use rand::{FromEntropy, RngCore, XorShiftRng, seq::sample_iter};

use blockchain::BlockChain;
use connection::{socket::Socket, {AddrsResponse, Connection, Disconnect, GetAddrsRequest}};

pub const DEFAULT_WATER_LINE: usize = 8;
pub const ADDR_POOL_SIZE: usize = 64;

pub const BITCOIN_DNS_SEEDS: [&'static str; 6] = [
    "seed.bitcoin.sipa.be",
    "dnsseed.bluematt.me",
    "dnsseed.bitcoin.dashjr.org",
    "seed.bitcoinstats,com",
    "bitseed.xf2.org",
    "seed.bitcoin.jonasschnelli.ch",
];

pub const TESTNET_DNS_SEEDS: [&'static str; 4] = [
    "testnet-seed.alexykot.me",
    "testnet-seed.bitcoin.petertodd.org",
    "testnet-seed.bluematt.me",
    "testnet-seed.bitcoin.schildbach.de",
];

pub const BITCOIN_PORT: u16 = 8333;
pub const TESTNET_PORT: u16 = 18333;

pub struct ConnectionPool
{
    connection_pool: HashSet<Addr<Connection>>,
    water_line: usize, // The number of connections it needs to keep
    addr_pool: Vec<SocketAddr>,

    rng: XorShiftRng,

    network: Network,
    services: u64,
    relay: bool,
    blockchain: Arc<Mutex<BlockChain>>,
}

#[derive(Message)]
#[rtype(result = "Vec<Addr<Connection>>")]
pub struct GetConnections
{
    pub num: usize,
    pub except: Vec<Addr<Connection>>,
}

#[derive(Message)]
pub struct BanConnection
{
    pub conn: Addr<Connection>,
}

impl Actor for ConnectionPool
{
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>)
    {
        self.feed_initial_addrs(ctx);
        ctx.run_interval(Duration::from_secs(30), |actor, ctx| {
            actor.health_check(ctx);
        });
    }
}

impl ConnectionPool
{
    pub fn new(network: Network, services: u64, relay: bool, blockchain: Arc<Mutex<BlockChain>>) -> ConnectionPool
    {
        ConnectionPool {
            connection_pool: HashSet::new(),
            water_line: DEFAULT_WATER_LINE,
            addr_pool: Vec::new(),

            rng: XorShiftRng::from_entropy(),

            network,
            services,
            relay,
            blockchain,
        }
    }

    fn add_connection(&mut self, addr: &SocketAddr, ctx: &mut Context<Self>)
    {
        let f = Socket::connect(addr, self.network)
            .into_actor(self)
            .and_then(|socket, actor, _ctx| {
                let start_height = {
                    let lock = actor.blockchain.lock().unwrap();
                    let active_chain = lock.active_chain();
                    let start_height = active_chain.latest_block().height();
                    start_height
                };
                socket
                    .begin_handshake(start_height as i32, actor.services, actor.relay)
                    .into_actor(actor)
            })
            .map(|socket, actor, ctx| {
                let conn = Connection::start_actor(socket);

                // Try send a GetAddrsRequest
                let me = ctx.address().recipient();
                let req = GetAddrsRequest { addr: me };
                conn.do_send(req);

                let _ = actor.connection_pool.insert(conn);
            })
            .map_err(|err, _actor, _ctx| {
                info!("Fail to establish connection : {:?}", err);
            });
        ctx.spawn(f);
    }

    // This function is called regulerly.
    // So even if connection_pool gets empty, it does not invoke recovery process immediately.
    fn health_check(&mut self, ctx: &mut Context<Self>)
    {
        // Remove all dropped connections
        self.connection_pool.retain(|addr| addr.connected());

        // If address pool is empty, we feed addresses to address pool but not try to establish a
        // new connection. It may happen in next cycle.
        if self.addr_pool.is_empty() {
            self.feed_initial_addrs(ctx);

        // If we does not have enough connection, we will try to establish a new connection.
        // Note that only one connection is tried to establish in one cycle.
        } else if !self.has_enough_connection() {
            let next_idx = self.rng.next_u32() as usize % self.addr_pool.len();
            let addr = self.addr_pool.swap_remove(next_idx);
            self.add_connection(&addr, ctx);
        }
    }

    fn has_enough_connection(&self) -> bool
    {
        self.water_line <= self.connection_pool.len()
    }

    fn feed_initial_addrs(&mut self, ctx: &mut Context<Self>)
    {
        let seeds = match self.network {
            Network::Bitcoin => &BITCOIN_DNS_SEEDS[..],
            Network::Testnet => &TESTNET_DNS_SEEDS[..],
            Network::Regtest => return,
        };
        let f = query_dns_seeds(&seeds)
            .into_actor(self)
            .map(|ips, actor, _ctx| {
                let port = match actor.network {
                    Network::Bitcoin => BITCOIN_PORT,
                    Network::Testnet => TESTNET_PORT,
                    Network::Regtest => unreachable!(),
                };
                for ip in ips {
                    actor.addr_pool.push(SocketAddr::new(ip, port));
                }
            })
            .map_err(|e, _actor, ctx| {
                info!("Could not query dns seed : {:?}", e);
                ctx.stop();
            });
        ctx.wait(f);
    }
}

impl Handler<AddrsResponse> for ConnectionPool
{
    type Result = ();

    fn handle(&mut self, msg: AddrsResponse, _ctx: &mut Context<Self>)
    {
        for (_ts, addr) in msg.0 {
            if self.addr_pool.len() > ADDR_POOL_SIZE {
                return;
            }
            if let Ok(a) = addr.socket_addr() {
                self.addr_pool.push(a);
            }
        }
    }
}

impl Handler<GetConnections> for ConnectionPool
{
    type Result = MessageResult<GetConnections>;

    fn handle(&mut self, msg: GetConnections, _ctx: &mut Context<Self>) -> MessageResult<GetConnections>
    {
        let iter = self.connection_pool
            .iter()
            .filter(|addr| !msg.except.contains(addr))
            .cloned();
        let vec = sample_iter(&mut self.rng, iter, msg.num).unwrap_or_else(|v| v);
        MessageResult(vec)
    }
}

impl Handler<BanConnection> for ConnectionPool
{
    type Result = ();

    fn handle(&mut self, msg: BanConnection, _ctx: &mut Context<Self>)
    {
        if let Some(conn) = self.connection_pool.take(&msg.conn) {
            // Even if it fail to send Disconnect message, if all Addr are dropped, underlying
            // Connection will stop.
            conn.do_send(Disconnect());
        }
    }
}

fn query_dns_seeds(seeds: &'static [&'static str]) -> Box<Future<Item = Vec<IpAddr>, Error = ResolveError>>
{
    let f = ResolverFuture::new(ResolverConfig::google(), ResolverOpts::default())
        .and_then(move |resolver| {
            let resolve_fut_iter = seeds.iter().map(move |seed| resolver.lookup_ip(*seed));
            ::futures::future::join_all(resolve_fut_iter)
        })
        .map(|vec_ips| vec_ips.iter().flat_map(|ips| ips.iter()).collect::<Vec<_>>());
    Box::new(f)
}
