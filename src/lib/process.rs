use connection::{Connection, IncomingMessage};
use node::{Node, ProcessResult};

pub fn process(conn: Connection, node: &mut Node)
{
    ::std::iter::repeat(()).try_fold(conn, |conn, _| single_process(conn, node));
}

fn single_process(mut conn: Connection, node: &mut Node) -> Option<Connection>
{
    let recv_result = conn.recv_msg();
    let process_result = match recv_result {
        Ok(IncomingMessage::Inv(invs)) => node.recv_inv(invs, &mut conn),
        Ok(IncomingMessage::Block(block)) => node.recv_block(block, &mut conn),
        Err(e) => {
            warn!("Error while receive message : {:?}", e);
            warn!("Drop connection {:?}", conn);
            return None;
        },
    };

    match process_result {
        ProcessResult::Ack => Some(conn),
        ProcessResult::Ban => {
            warn!("Drop connection");
            None
        },
    }
}
