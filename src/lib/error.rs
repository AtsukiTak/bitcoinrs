error_chain! {
    types {
        Error, ErrorKind, ResultExt;
    }

    foreign_links {
        Bitcoin(::bitcoin::util::Error);
        Io(::std::io::Error);
    }

    errors {
        HandshakeError(socket: ::socket::AsyncSocket) {
            description("Error while handshaking")
            display("Error while handshaking on {:?}", socket)
        }
        MisbehaviorPeer(conn: ::connection::Connection) {
            description("Misbehavior peer")
            display("Peer {} does misbehavior", conn)
        }
    }
}
