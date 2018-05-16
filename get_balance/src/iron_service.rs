// Copied from https://github.com/debitinc/api_http_jsonrpc/blob/develop/src/iron_service.rs

use std::fmt;
use std::error::Error;
use std::io::Read;
use std::sync::Arc;

use futures::Future;
use iron::prelude::*;
use iron::method::Method;
use iron::{status, Handler};
use jsonrpc_core::MetaIoHandler;
use grpcio::Channel;

use hmac_authenticator_proto::service::AuthRequest;
use hmac_authenticator_proto::service_grpc::AuthenticationRpcClient;

use jsonrpc_handlers::Access;

macro_rules! get_header {
    ($header:expr, $field:expr) => {
        match $header.get_raw($field) {
            Some(ref bytes) => Ok(String::from_utf8(bytes[0].clone()).unwrap()),
            None => Err(IronError::new(AuthMissingHeader::new($field), status::Forbidden)),
        }
    };
}

pub struct JsonRpc {
    handler: Arc<MetaIoHandler<Access>>,
    validator: AuthenticationRpcClient,
}

impl JsonRpc {
    pub fn new(
        handler: MetaIoHandler<Access>,
        grpc_channel: Channel,
    ) -> JsonRpc {
        JsonRpc {
            handler: Arc::new(handler),
            validator: AuthenticationRpcClient::new(grpc_channel),
        }
    }
}

impl Handler for JsonRpc {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        trace!("Received client request: {:?}", &req);
        
        let mut full_path = "".to_owned();
        for segment in req.url.path().iter() {
            full_path = format!("{}/{}", &full_path, segment);
        }
        let path = &full_path[..];
        match (&req.method, path) {
            (&Method::Post, "/jsonrpc/v1") => {
                let mut json = String::new();
                let _ = req.body.read_to_string(&mut json).unwrap();
                let key = get_header!(req.headers, "X-Access-Id")?;                

                let key: u64 = match key.parse() {
                    Ok(i) => i,
                    Err(_) => return Ok(Response::with(
                        status::NetworkAuthenticationRequired)),
                };
                
                let sig = get_header!(req.headers, "X-Signature")?;
                let nonce = get_header!(req.headers, "X-Nonce")?;
                
                let nonce: u64 = match nonce.parse() {
                    Ok(n) => n,
                    Err(_) => return Ok(Response::with(
                        status::NetworkAuthenticationRequired)),
                };

                // Construct request object.
                let mut a_req = AuthRequest::new();
                a_req.set_key(key);
                a_req.set_sig(sig);
                a_req.set_body(json.clone());
                a_req.set_nonce(nonce);

                // validate!
                let a_rsp = self.validator.authentication(&a_req)
                    .map_err(|e| {
                        error!("gRPC connection with hmac_authenticator lost!: {}", &e);
                        IronError::new(InternalServerError, status::InternalServerError)
                    })?;

                if a_rsp.has_valid() {
                    let access = Access::new(a_rsp.get_valid());
                    let response = self.handler.handle_request(&json, access)
                        .wait()
                        .unwrap()
                        .unwrap();
                    Ok(Response::with((status::Ok, response)))
                } else {
                    Ok(Response::with(status::NetworkAuthenticationRequired))
                }
            },
            _ => Ok(Response::with(status::NetworkAuthenticationRequired))
        }
    } 
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthMissingHeader {
    description: String,
}

impl AuthMissingHeader {
    pub fn new(header: &str) -> AuthMissingHeader {
        AuthMissingHeader {
            description: format!("Missing header {}", &header),
        }
    }
}

impl Error for AuthMissingHeader {
    fn description(&self) -> &str {
        &self.description
    }
}

impl fmt::Display for AuthMissingHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", &self.description)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct InternalServerError;

impl Error for InternalServerError {
    fn description(&self) -> &str {
        "Internal Server Error."
    }
}

impl fmt::Display for InternalServerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Internal Server Error.")
    }
}