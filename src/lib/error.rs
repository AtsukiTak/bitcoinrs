error_chain! {
    types {
        Error, ErrorKind, ResultExt;
    }

    foreign_links {
        Bitcoin(::bitcoin::util::Error);
        Io(::std::io::Error);
    }

    errors {
        InvalidPeer {
            description("Invalid peer")
            display("Invalid peer")
        }
    }
}
