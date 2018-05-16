// Copied from https://github.com/debitinc/api_http_jsonrpc/blob/develop/src/config.rs

use std::{fmt, net, env};
use std::error::Error;
use std::convert::From;
use std::net::SocketAddr;
use std::path::PathBuf;

use hyper_openssl::OpensslServer;
use dotenv;

pub struct Configuration {
    listen: SocketAddr,
    ssl: Option<OpensslServer>,

    cs_grpc: String,
    auth_grpc: String,
}

impl Configuration {
    pub fn new<C, A>(
        listen: SocketAddr,
        ssl: Option<OpensslServer>,
        authenticator: A,
        client_service: C
    ) -> Configuration
        where A: Into<String>,
              C: Into<String>,
    {
        Configuration {
            listen: listen,
            ssl: ssl,
            cs_grpc: client_service.into(),
            auth_grpc: authenticator.into(),
        }
    }

    pub fn consume(self) -> (SocketAddr, Option<OpensslServer>, String, String) {
        (self.listen, self.ssl, self.cs_grpc, self.auth_grpc)
    }
}

pub fn from_environment() -> Result<Configuration, LoadConfigError> {
    dotenv::dotenv().ok();

    let listen = env::var("API_HTTP_JSONRPC_LISTEN")?;
    let listen: SocketAddr = listen.parse()?;

    // Load SSL settings 
    let ssl = if let Ok(k) = env::var("API_HTTP_JSONRPC_SSL_KEY") {
        if let Ok(c) = env::var("API_HTTP_JSONRPC_SSL_CERT") {
            let key: PathBuf = k.into();
            let cert: PathBuf = c.into();
            let ssl = OpensslServer::from_files(key, cert).unwrap();
            
            info!("SSL settings loaded");
                
            Some(ssl)
        } else {
            None
        }
    } else {
        None
    };

    let config = Configuration {
        listen: listen,
        ssl: ssl,
        cs_grpc: env::var("CLIENT_SERVICE_GRPC")?,
        auth_grpc: env::var("HMAC_AUTHENTICATOR_GRPC")?,
    };

    Ok(config)
}

/// Specific errors that may happen with loading the configuration.
#[derive(Debug)]
pub enum LoadConfigError {
    Dotenv(dotenv::Error),
    Env(env::VarError),
    Addr(net::AddrParseError),
}

impl fmt::Display for LoadConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &LoadConfigError::Dotenv(ref err) => write!(f, "Bad .env: {}", &err),
            &LoadConfigError::Env(ref err) => write!(f, "Bad environemtn: {}", &err),
            &LoadConfigError::Addr(ref err) => write!(f, "Address error: {}", &err),
        }
    }
}

impl Error for LoadConfigError {
    fn description(&self) -> &str {
        "Error with loading API server configuration"
    }
}

impl From<dotenv::Error> for LoadConfigError {
    fn from(err: dotenv::Error) -> Self {
        LoadConfigError::Dotenv(err)
    }
}

impl From<net::AddrParseError> for LoadConfigError {
    fn from(err: net::AddrParseError) -> Self {
        LoadConfigError::Addr(err)
    }
}

impl From<env::VarError> for LoadConfigError {
    fn from(err: env::VarError) -> Self {
        LoadConfigError::Env(err)
    }
}