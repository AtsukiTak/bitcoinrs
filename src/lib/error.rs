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
        MisbehavePeer
    }
}
