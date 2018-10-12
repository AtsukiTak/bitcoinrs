pub struct Connection
{
    executor: TaskExecutor,
    sending_socket: Socket<S>,
}

impl Connection
{
    pub fn new(socket: HandshakedSocket<S>, executor: TaskExecutor) -> Connection
    {
        let (read_socket, write_socket) = socket.split();

        let msg_stream = kk
    }
}
