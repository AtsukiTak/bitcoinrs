use std::sync::{Arc, Mutex};
use futures::{Stream, sync::oneshot::{self, SpawnHandle}};
use tokio::{io::{ReadHalf, WriteHalf}, net::TcpStream, runtime::TaskExecutor};
use bitcoin::network::{message::NetworkMessage, message_blockdata::GetHeadersMessage};
use bitcoin::blockdata::block::LoneBlockHeader;
use bitcoin::util::hash::Sha256dHash;

use connection::{ConnectionError, socket::Socket};
use failure::Error;

pub struct Connection
{
    inner: Arc<Mutex<ConnectionInner>>,

    // Stop receiving loop when `SpawnHandle` is dropped.
    _recv_process_handle: SpawnHandle<(), Error>,
}

impl Connection
{
    pub fn new(socket: Socket<TcpStream>, executor: &TaskExecutor) -> Connection
    {
        let (read_socket, write_socket) = socket.split();

        let inner = Arc::new(Mutex::new(ConnectionInner::new(write_socket)));

        let recv_process_handle = recv_process(read_socket, &inner, &executor);

        Connection {
            inner,
            _recv_process_handle: recv_process_handle,
        }
    }

    pub fn getheaders(&self, locator_hashes: Vec<Sha256dHash>)
        -> Result<oneshot::Receiver<Vec<LoneBlockHeader>>, Error>
    {
        let (tx, rx) = oneshot::channel();
        let getheaders = GetHeadersMessage::new(locator_hashes, Sha256dHash::default());
        let msg = NetworkMessage::GetHeaders(getheaders);
        {
            let mut inner = self.inner.lock().unwrap();
            inner.send_p2p_msg(msg)?;
            inner.waiting_headers = Some(tx);
        }
        Ok(rx)
    }
}

struct ConnectionInner
{
    sending_socket: Socket<WriteHalf<TcpStream>>,

    waiting_headers: Option<oneshot::Sender<Vec<LoneBlockHeader>>>,
}

impl ConnectionInner
{
    fn new(socket: Socket<WriteHalf<TcpStream>>) -> ConnectionInner
    {
        ConnectionInner {
            sending_socket: socket,

            waiting_headers: None,
        }
    }

    fn send_p2p_msg(&mut self, msg: NetworkMessage) -> Result<(), Error>
    {
        self.sending_socket.sync_send_msg(msg)
    }
}

/* Handle NetworkMessage */
impl ConnectionInner
{
    fn handle_network_msg(&mut self, msg: NetworkMessage) -> Result<(), Error>
    {
        use self::NetworkMessage::*;
        match msg {
            // Addr(addrs) => self.handle_addr_msg(addrs),
            // Inv(invs) => self.handle_inv_msg(invs),
            // Block(block) => self.handle_block_msg(block),
            Headers(headers) => self.handle_headers_msg(headers),
            Ping(nonce) => self.handle_ping_msg(nonce),
            another => {
                debug!("Discard unexpected msg {:?}", another);
                Ok(())
            },
        }
    }

    fn handle_headers_msg(&mut self, headers: Vec<LoneBlockHeader>) -> Result<(), Error>
    {
        let maybe_waiting_headers = self.waiting_headers.take();
        match maybe_waiting_headers {
            None => {
                debug!("Receive unexpected headers msg");
                Err(Error::from(ConnectionError::MisbehavePeer))
            },
            Some(waiting_headers) => {
                let _ = waiting_headers.send(headers);
                Ok(())
            },
        }
    }

    fn handle_ping_msg(&mut self, nonce: u64) -> Result<(), Error>
    {
        let pong = NetworkMessage::Pong(nonce);
        self.send_p2p_msg(pong)
    }
}

fn recv_process(
    socket: Socket<ReadHalf<TcpStream>>,
    inner: &Arc<Mutex<ConnectionInner>>,
    executor: &TaskExecutor,
) -> SpawnHandle<(), Error>
{
    let inner2 = inner.clone();
    let f = socket
        .recv_msg_stream()
        .for_each(move |msg| inner2.lock().unwrap().handle_network_msg(msg));
    oneshot::spawn(f, executor)
}
