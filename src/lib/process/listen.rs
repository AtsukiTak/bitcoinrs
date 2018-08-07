use connection::Connection;
use super::ProcessError;

pub fn listen_new_block(conn: Connection) -> impl Stream<Item = Block, Error = ProcessError>
{
}

fn wait_recv_block(conn: Connection) {
    conn.recv_msg().and_then(|(msg, conn)| {
        match msg {
            // If we use "standard block relay", peer sends "inv" message first.
            // Or even if we have signalled "sendheaders", peer may send "inv" message first.
            IncomingMessage::Inv(invs) => Hoge,

            // If we have signalled "sendheaders", we may use "direct headers announcement".
            // In that case, peer may send "headers" message instead of "inv" message.
            IncomingMessage::Headers(headers) => hoge,
        }
    })
}
