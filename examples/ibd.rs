extern crate bitcoin;
extern crate futures;
extern crate tokio;

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate libyabitcoin;

use bitcoin::network::{constants::Network, serialize::BitcoinHash};
use bitcoin::blockdata::block::{Block, BlockHeader};
use bitcoin::util::hash::Sha256dHash;

use futures::Future;

use libyabitcoin::{blockchain::{BlockChainMut, HeaderOnlyBlock, StoredBlock}, connection::Connection,
                   process::initial_block_download, socket::AsyncSocket};

const DEMO_PEER: &str = "172.105.194.235:8333";
// const DEMO_PEER: &str = "35.187.215.241:8333";
const LOCAL_PEER: &str = "10.0.1.16:8333";

fn main()
{
    env_logger::init();

    let ibd_fut = AsyncSocket::open(&DEMO_PEER.parse().unwrap(), Network::Bitcoin)
        .and_then(|socket| Connection::initialize(socket, 0))
        .and_then(|conn| {
            info!("Connected");
            let start_block = HeaderOnlyBlock::new(start_block());
            let blockchain = BlockChainMut::with_start(start_block);
            initial_block_download(conn, blockchain)
        })
        .map_err(|e| {
            error!("{:?}", e);
        })
        .map(|res| {
            info!("Complete ibd process!!");
        });

    let mut executor = ::tokio::executor::current_thread::CurrentThread::new();
    executor.spawn(ibd_fut).run().unwrap();
}

fn start_block() -> Block
{
    const START_BLOCK_HASH: &str = "000000000000000000376b62d61208a7e45a030c6b876e3516083bdd62be4097";
    const START_BLOCK_PREV_HASH: &str = "0000000000000000001f5dee17110cb311de968496c0813918b15a9ff239c75e";
    const START_BLOCK_MERKLE_ROOT: &str = "2c555f43f0588b73f23c806e821d39a0c035985917aaeb20e9ae4c993d730f9a";

    let header = BlockHeader {
        version: 536870912,
        prev_blockhash: Sha256dHash::from_hex(START_BLOCK_PREV_HASH).unwrap(),
        merkle_root: Sha256dHash::from_hex(START_BLOCK_MERKLE_ROOT).unwrap(),
        time: 1530447144,
        bits: 389508950,
        nonce: 449341550,
    };

    assert_eq!(Sha256dHash::from_hex(START_BLOCK_HASH).unwrap(), header.bitcoin_hash());

    Block {
        header,
        txdata: Vec::new(),
    }
}
