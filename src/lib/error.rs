error_chain! {
    types {
        Error, ErrorKind, ResultExt;
    }

    foreign_links {
        BitcoinNetwork(::bitcoin::network::Error);
        BitcoinUtil(::bitcoin::util::Error);
        BitcoinSerialize(::bitcoin::network::serialize::Error);
        Io(::std::io::Error);
    }

    errors {
        HandshakeError(socket: ::peer::Socket) {
            description("Error while handshaking")
            display("Error while handshaking on {:?}", socket)
        }
        MisbehaviorPeer(conn: ::peer::Connection) {
            description("Misbehavior peer")
            display("Peer {} does misbehavior", conn)
        }
    }
}
