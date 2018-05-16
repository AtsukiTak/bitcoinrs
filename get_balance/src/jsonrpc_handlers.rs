// Copied from https://github.com/debitinc/api_http_jsonrpc/blob/develop/src/jsonrpc_handlers.rs

//! JSONRPC V2.0 handlers.

use std::str::FromStr;
//use std::sync::Arc;

use jsonrpc_core::*;
use jsonrpc_core::params::Params;
use jsonrpc_core::types::{Value, Error, to_value};
use futures::{future, Future};
use grpcio::Channel;

use precision::Dec;
use market_types::order;

use client_service_proto::proto::client_service_grpc::{
    AccountInfoRpcClient,
    AccountBalanceRpcClient,
    AccountBtcDepositAddressRpcClient,
    AccountWithdrawalRpcClient,
    AccountSearchWithdrawalRpcClient,
    AccountSearchTransactionRpcClient,
    TradeMarketPricesRpcClient,
    TradeOrderRpcClient
};
use client_service_proto::proto::client_service::*;
use model::{btc_deposit_address, deposit_record, market_prices, transaction, ticks, trade};
use model::info::info_from_protobuf;
use model::balance::balances_from_protobuf;
use model::withdrawal::{self, record_from_protobuf};

lazy_static! {
    static ref SATOSHI: Dec = Dec::from_str("0.00000001").unwrap();
}

macro_rules! parse_param {
    ($param:expr) => {
        match $param.parse() {
            Ok(d) => d,
            Err(e) => return Box::new(future::err(e)),
        }
    }
}

macro_rules! size_satoshi {
    ($size:expr) => {
        if $size < *SATOSHI {
            let mut error = Error::new(ErrorCode::ServerError(12));
            error.message = "Size is less than one satoshi.".to_string();
            return Box::new(future::err(error));
        }
    }
}

macro_rules! place_order {
    ($norder:expr, $rpc:expr) => {
        let norder: NewOrder = $norder.into();
        
        let results = $rpc.place_order(&norder).expect("RPC disconnected.");

        if results.has_ok() {
            let order: trade::Order = results.get_ok().into();
            return Box::new(future::ok(to_value(order).unwrap()));
        } else {
            let error: trade::OrderError = results.get_err().into();
            return Box::new(future::err(error.get_jsonrpc_error()));
        }
    }
}

#[derive(Debug, Default, Copy, Clone, Getters)]
pub struct Access {
    account_id: u64,
}

impl Access {
    pub fn new(account: u64) -> Access {
        Access {
            account_id: account,
        }
    }
}

impl Metadata for Access { }

/// Fetch client account information. Corresponds to API call detailed in
struct AccountInfo {
    rpc: AccountInfoRpcClient,
}

impl RpcMethod<Access> for AccountInfo {
    fn call(&self, _: Params, meta: Access)
            -> Box<Future<Item = Value, Error = Error> + Send + 'static> {
        debug!("Client {} requested account info.", meta.account_id());
        let mut req = AccountInfoRequest::new();
        req.set_account_id(meta.account_id);
        let rsp = self.rpc.client_account_info(&req).expect("RPC disconnected.");

        match info_from_protobuf(&rsp) {
            Ok(info) => {
                let info = to_value(&info).unwrap();
                Box::new(future::ok(info))
            },
            Err(_) => {
                // TODO: Log the error (and send notification?)
                Box::new(future::err(Error::internal_error()))
            }
        }       
    }
}

struct AccountBalance {
    rpc: AccountBalanceRpcClient,
}

impl RpcMethod<Access> for AccountBalance {
    fn call(
        &self, _: Params,
        meta: Access
    ) -> Box<Future<Item = Value, Error = Error> + Send + 'static> {
        debug!("Client {} requested account balance.", meta.account_id());
        let mut req = AccountBalanceRequest::new();
        req.set_account_id(meta.account_id);
        let rsp = self.rpc.client_account_balance(&req).expect("RPC disconnected.");

        match balances_from_protobuf(&rsp) {
            Ok(bals) => {
                let bals = to_value(&bals).unwrap();
                Box::new(future::ok(bals))
            },
            Err(_) => {
                // TODO: Log the error (and send notification?)
                Box::new(future::err(Error::internal_error()))
            },
        }
    }
}

struct AccountBtcDepositAddress {
    rpc: AccountBtcDepositAddressRpcClient,
}

impl RpcMethod<Access> for AccountBtcDepositAddress {
    fn call(&self, _: Params, meta: Access)
            -> Box<Future<Item = Value, Error = Error> + Send + 'static> {
        debug!("Client {} requested bitcoin deposit address.", meta.account_id());
        let mut req = AccountBtcDepositAddressRequest::new();
        req.set_account_id(meta.account_id);
        let rsp = self.rpc.get_btc_deposit_address(&req).expect("RPC disconnected.");

        let addr = btc_deposit_address::BtcDepositAddress::new(rsp.get_address());
        Box::new(future::ok(to_value(&addr).unwrap()))
    }
}

struct AccountDepositSearchRecord;

impl RpcMethod<Access> for AccountDepositSearchRecord {
    fn call(&self, param: Params, meta: Access)
            -> Box<Future<Item = Value, Error = Error> + Send + 'static> {
        debug!("Client {} is searching for a deposit record.", meta.account_id());
        let criteria: deposit_record::DepositSearchCriteria = parse_param!(param);

        if criteria.empty() {
            let mut error = Error::new(ErrorCode::ServerError(1));
            error.message = "No criteria supplied.".to_string();
            return Box::new(future::err(error));
        }

        // STUB
        let empty: Vec<String> = Vec::new();
        let empty = to_value(empty).unwrap();
        Box::new(future::ok(empty))
    }
}

struct AccountBtcWithdrawal {
    rpc: AccountWithdrawalRpcClient,
}

impl RpcMethod<Access> for AccountBtcWithdrawal {
    fn call(&self, param: Params, meta: Access)
            -> Box<Future<Item = Value, Error = Error> + Send + 'static> {
        debug!("Client {} is requesting to withdraw bitcoin.", meta.account_id());
        let request: withdrawal::BtcWithdrawalRequest = parse_param!(param);

        let rsp = self.rpc.withdraw_btc(&request.into()).expect("RPC disconnected.");

        if rsp.has_ok() {
            let record = record_from_protobuf(rsp.get_ok()).unwrap();
            let record = to_value(record).unwrap();
            Box::new(future::ok(record))
        } else {
            let mut error = Error::new(ErrorCode::ServerError(4));
            error.message = rsp.get_error().to_owned();
            Box::new(future::err(error))
        }
    }
}

struct AccountFiatWithdrawal {
    rpc: AccountWithdrawalRpcClient,
}

impl RpcMethod<Access> for AccountFiatWithdrawal {
    fn call(&self, param: Params, meta: Access)
            -> Box<Future<Item = Value, Error = Error> + Send + 'static> {
        debug!("Client {} is requesting to withdraw fiat.", meta.account_id());
        let request: withdrawal::FiatWithdrawalRequest = parse_param!(param);

        let rsp = self.rpc.withdraw_fiat(&request.into()).expect("RPC disconnected.");

        if rsp.has_ok() {
            let record = record_from_protobuf(rsp.get_ok()).unwrap();
            let record = to_value(record).unwrap();
            Box::new(future::ok(record))
        } else {
            let mut error = Error::new(ErrorCode::ServerError(4));
            error.message = rsp.get_error().to_owned();
            Box::new(future::err(error))
        }
    }
}

struct AccountTransferMoney;

impl RpcMethod<Access> for AccountTransferMoney {
    fn call(&self, request: Params, meta: Access)
            -> Box<Future<Item = Value, Error = Error> + Send + 'static> {
        debug!("Client {} is requesting to transfer money.", meta.account_id());
        let _request: withdrawal::TransferMoneyRequest = parse_param!(request);

        // TODO
        // Implement this method.

        let mut error = Error::new(ErrorCode::ServerError(5));
        error.message = "Operation not authorized. No linked accounts".to_string();
        Box::new(future::err(error))
    }
}

struct AccountSearchWithdrawalRecord {
    rpc: AccountSearchWithdrawalRpcClient,
}

impl RpcMethod<Access> for AccountSearchWithdrawalRecord {
    fn call(&self, request: Params, meta: Access)
            -> Box<Future<Item = Value, Error = Error> + Send + 'static> {
        debug!("Client {} is searching for a withdrawal record.", meta.account_id());
        let criteria: withdrawal::WithdrawalSearchCriteria = parse_param!(request);

        if criteria.empty() {
            let mut error = Error::new(ErrorCode::ServerError(1));
            error.message = "No criteria supplied.".to_string();
            return Box::new(future::err(error));
        }

        let mut criteria: AccountWithdrawalSearchCriteria = criteria.into();
        criteria.set_account_id(*meta.account_id());

        let rsp = self.rpc.search_withdrawal(&criteria).expect("RPC disconnected.");

        let records = rsp.get_records()
            .iter()
            .map(|r| record_from_protobuf(r).unwrap())
            .collect::<Vec<withdrawal::WithdrawalRecord>>();

        let records = to_value(&records).unwrap();
        Box::new(future::ok(records))
    }
}

/// AKA account_get_transaction
struct AccountSearchTransaction {
    rpc: AccountSearchTransactionRpcClient,
}

impl RpcMethod<Access> for AccountSearchTransaction {
    fn call(&self, param: Params, meta: Access)
            -> Box<Future<Item = Value, Error = Error> + Send + 'static> {
        debug!("Client {} is searching for a cashflow transaction.", meta.account_id());
        let criteria: transaction::TransactionSearchCriteria = parse_param!(param);

        if criteria.empty() {
            let mut error = Error::new(ErrorCode::ServerError(7));
            error.message = "No criteria supplied.".to_string();
            return Box::new(future::err(error));
        }

        let mut criteria: AccountTransactionSearchCriteria = criteria.into();
        criteria.set_account_id(*meta.account_id());

        let rsp = self.rpc.search_transaction(&criteria).expect("RPC disconnected.");

        let records = rsp.get_transactions()
            .iter()
            .map(|r| transaction::from_protobuf(r).unwrap())
            .collect::<Vec<transaction::Transaction>>();

        let records = to_value(&records).unwrap();
        Box::new(future::ok(records))
    }
}

struct TradeMarketPrices {
    rpc: TradeMarketPricesRpcClient,
}

impl RpcMethod<Access> for TradeMarketPrices {
    fn call(&self, param: Params, meta: Access)
            -> Box<Future<Item = Value, Error = Error> + Send + 'static> {
        debug!("Client {} wants trade market prices.", meta.account_id());
        let param: market_prices::MarketPriceCriteria = parse_param!(param);

        let criteria: TradeMarketPricesCriteria = param.into();

        let rsp = self.rpc.get_prices(&criteria).expect("RPC disconnected.");

        let prices = rsp.get_prices()
            .iter()
            .map(|p| p.into())
            .collect::<Vec<market_prices::MarketPrices>>();

        let prices = to_value(&prices).unwrap();
        Box::new(future::ok(prices))
    }
}

struct TradeLockPrices {
    rpc: TradeMarketPricesRpcClient,
}

impl RpcMethod<Access> for TradeLockPrices {
    fn call(&self, param: Params, meta: Access)
            -> Box<Future<Item = Value, Error = Error> + Send + 'static> {
        debug!("Client {} wants to lock some prices.", meta.account_id());
        let params: market_prices::LockPriceRequest = parse_param!(param);

        let mut request: TradeLockPriceRequest = params.into();
        request.set_account_id(*meta.account_id());

        let rsp = self.rpc.lock_prices(&request).expect("RPC disconnected.");

        let results = rsp.get_locks()
            .iter()
            .map(|l| l.into())
            .collect::<Vec<market_prices::PriceLockResult>>();
        
        let results = to_value(&results).unwrap();

        Box::new(future::ok(results))
    }
}

struct TradeGetTicks;

impl RpcMethod<Access> for TradeGetTicks {
    fn call(&self, param: Params, meta: Access)
            -> Box<Future<Item = Value, Error = Error> + Send + 'static> {
        debug!("Client {} is looking to get trade ticks.", meta.account_id());
        let _criteria: ticks::TradeTicksCriteriaRaw = parse_param!(param);

        let empty: Vec<u8> = Vec::new();
        let empty = to_value(empty).unwrap();
        Box::new(future::ok(empty))
    }
}

struct TradePlaceOrder {
    rpc: TradeOrderRpcClient,
}

impl RpcMethod<Access> for TradePlaceOrder {
    fn call(&self, param: Params, meta: Access)
            -> Box<Future<Item = Value, Error = Error> + Send + 'static> {
        debug!("Client {} is placing an order.", meta.account_id());
        let place: trade::PlaceOrder = parse_param!(param);
        size_satoshi!(place.size);

        // We check if this is a price lock order
        if let Some(_id) = place.price_lock_id {
            // TODO: Fetch the price lock.
            let mut error = Error::new(ErrorCode::ServerError(2));
            error.message = "Lock record not found.".into();
            return Box::new(future::err(error)); 
        }

        let norder = trade::NewOrder::new(*meta.account_id(),
                                          None,
                                          place.asset_pair,
                                          place.side,
                                          place.order_type,
                                          place.size,
                                          place.price,
                                          None,
                                          place.tracking_code,
                                          place.wait);
        place_order!(norder, self.rpc);
    }
}

struct TradePlaceMarket {
    side: order::Side,
    rpc: TradeOrderRpcClient,
}

impl RpcMethod<Access> for TradePlaceMarket {
    fn call(&self, param: Params, meta: Access)
            -> Box<Future<Item = Value, Error = Error> + Send + 'static> {
        debug!("Client {} is placing a market FaS order.", meta.account_id());
        let place: trade::PlaceMarket = parse_param!(param);
        size_satoshi!(place.size);

        let norder: trade::NewOrder = (*meta.account_id(), self.side, place).into();
        place_order!(norder, self.rpc);
    }
}

struct TradePlaceMarketFak {
    side: order::Side,
    rpc: TradeOrderRpcClient,
}

impl RpcMethod<Access> for TradePlaceMarketFak {
    fn call(&self, param: Params, meta: Access)
            -> Box<Future<Item = Value, Error = Error> + Send + 'static> {
        debug!("Client {} is placing a market FaK order.", meta.account_id());
        let place: trade::PlaceMarketFak = parse_param!(param);
        size_satoshi!(place.size);

        let norder: trade::NewOrder = (*meta.account_id(), self.side, place).into();
        place_order!(norder, self.rpc);
    }
}

struct TradePlaceMarketFok {
    side: order::Side,
    rpc: TradeOrderRpcClient,
}

impl RpcMethod<Access> for TradePlaceMarketFok {
    fn call(&self, param: Params, meta: Access)
            -> Box<Future<Item = Value, Error = Error> + Send + 'static> {
        debug!("Client {} is placing a market FoK order.", meta.account_id());
        let place: trade::PlaceMarketFok = parse_param!(param);
        size_satoshi!(place.size);

        let norder: trade::NewOrder = (*meta.account_id(), self.side, place).into();
        place_order!(norder, self.rpc);
    }
}

struct TradePlaceLimit {
    side: order::Side,
    rpc: TradeOrderRpcClient,
}

impl RpcMethod<Access> for TradePlaceLimit {
    fn call(&self, param: Params, meta: Access)
            -> Box<Future<Item = Value, Error = Error> + Send + 'static> {
        debug!("Client {} is placing a limit FaS order.", meta.account_id());
        let place: trade::PlaceLimit = parse_param!(param);
        size_satoshi!(place.size);

        let norder: trade::NewOrder = (*meta.account_id(), self.side, place).into();
        place_order!(norder, self.rpc);
    }
}

struct TradePlaceLimitFak {
    side: order::Side,
    rpc: TradeOrderRpcClient,
}

impl RpcMethod<Access> for TradePlaceLimitFak {
    fn call(&self, param: Params, meta: Access)
            -> Box<Future<Item = Value, Error = Error> + Send + 'static> {
        debug!("Client {} is placing a limit FaK order.", meta.account_id());
        let place: trade::PlaceLimitFak = parse_param!(param);
        size_satoshi!(place.size);

        let norder: trade::NewOrder = (*meta.account_id(), self.side, place).into();
        place_order!(norder, self.rpc);
    }
}

struct TradePlaceStop {
    side: order::Side,
    rpc: TradeOrderRpcClient,
}

impl RpcMethod<Access> for TradePlaceStop {
    fn call(&self, param: Params, meta: Access)
            -> Box<Future<Item = Value, Error = Error> + Send + 'static> {
        debug!("Client {} is placing a stop order.", meta.account_id());
        let place: trade::PlaceStop = parse_param!(param);
        size_satoshi!(place.size);

        let norder: trade::NewOrder = (*meta.account_id(), self.side, place).into();
        place_order!(norder, self.rpc);
    }
}

struct TradeCancelOrder {
    rpc: TradeOrderRpcClient,
}

impl RpcMethod<Access> for TradeCancelOrder {
    fn call(&self, param: Params, meta: Access)
            -> Box<Future<Item = Value, Error = Error> + Send + 'static> {
        debug!("Client {} is cancelling an order.", meta.account_id());
        let criteria: trade::TradeCancelOrder = parse_param!(param);

        if criteria.is_empty() {
            let mut error = Error::new(ErrorCode::ServerError(7));
            error.message = "No criteria supplied.".to_string();
            return Box::new(future::err(error));
        }

        let mut request = CancelOrderRequest::new();
        request.set_account_id(*meta.account_id());
        
        if let Some(order_id) = criteria.order_id {
            request.set_order_id(order_id);
        }

        if let Some(code) = criteria.code {
            request.set_code(code)
        }

        let result = self.rpc.cancel_order(&request).expect("RPC disconnected.");

        if result.has_ok() {
            let order: trade::Order = result.get_ok().into();
            Box::new(future::ok(to_value(order).unwrap()))
        } else {
            let error: trade::OrderError = result.get_err().into();
            Box::new(future::err(error.get_jsonrpc_error()))
        }
    }
}

struct TradeSearchOrder {
    rpc: TradeOrderRpcClient,
}

impl RpcMethod<Access> for TradeSearchOrder {
    fn call(&self, param: Params, meta: Access)
            -> Box<Future<Item = Value, Error = Error> + Send + 'static> {
        debug!("Client {} is searching for an order.", meta.account_id());
        let criteria: trade::OrderSearchCriteria
            = parse_param!(param);
        debug!("Search criteria: {:?}", &criteria);

        if criteria.empty() {
            let empty: Vec<trade::Order> = Vec::with_capacity(0);
            let empty = to_value(empty).unwrap();
            return Box::new(future::ok(empty));
        }

        let mut criteria: OrderSearchCriteria = criteria.into();
        criteria.set_account_id(*meta.account_id());

        let result = self.rpc.search_order(&criteria).expect("RPC disconnected.");

        if result.has_ok() {
            let orders = result.get_ok();
            let matches = orders.get_order()
                .iter()
                .map(|o| o.into())
                .collect::<Vec<trade::Order>>();
            Box::new(future::ok(to_value(matches).unwrap()))
        } else {
            let error: trade::OrderError = result.get_err().into();
            Box::new(future::err(error.get_jsonrpc_error()))
        }
    }
}

pub fn prepare(grpc_channel: Channel) -> MetaIoHandler<Access> {
    let mut handler = MetaIoHandler::new(Compatibility::V2, NoopMiddleware::default());
    
    handler.add_method_with_meta("account_info", AccountInfo {
        rpc: AccountInfoRpcClient::new(grpc_channel.clone().clone()),
    });

    handler.add_method_with_meta("account_balance", AccountBalance {
        rpc: AccountBalanceRpcClient::new(grpc_channel.clone()),
    });

    handler.add_method_with_meta("account_btc_deposit_address", AccountBtcDepositAddress {
        rpc: AccountBtcDepositAddressRpcClient::new(grpc_channel.clone()),
    });

    handler.add_method_with_meta("account_deposit_record", AccountDepositSearchRecord);

    /*
    handler.add_method_with_meta("account_withdraw_btc", AccountBtcWithdrawal {
        rpc: AccountWithdrawalRpcClient::new(grpc_channel.clone()),
    });

    handler.add_method_with_meta("account_withdraw_fiat", AccountFiatWithdrawal {
        rpc: AccountWithdrawalRpcClient::new(grpc_channel.clone()),
    });
    */

    handler.add_method_with_meta(
        "account_withdrawal_record", AccountSearchWithdrawalRecord {
            rpc: AccountSearchWithdrawalRpcClient::new(grpc_channel.clone()),
        }
    );

    handler.add_method_with_meta("account_get_transaction", AccountSearchTransaction {
        rpc: AccountSearchTransactionRpcClient::new(grpc_channel.clone()),
    });

    handler.add_method_with_meta("account_transfer_money", AccountTransferMoney { });

    handler.add_method_with_meta("trade_market_prices", TradeMarketPrices {
        rpc: TradeMarketPricesRpcClient::new(grpc_channel.clone()),
    });

    handler.add_method_with_meta("trade_lock_prices", TradeLockPrices {
        rpc: TradeMarketPricesRpcClient::new(grpc_channel.clone()),
    });

    handler.add_method_with_meta("trade_place_order", TradePlaceOrder {
        rpc: TradeOrderRpcClient::new(grpc_channel.clone()),
    });

    handler.add_method_with_meta("trade_place_market_buy", TradePlaceMarket {
        side: order::Side::Buy,
        rpc: TradeOrderRpcClient::new(grpc_channel.clone()),
    });

    handler.add_method_with_meta("trade_place_market_sell", TradePlaceMarket {
        side: order::Side::Sell,
        rpc: TradeOrderRpcClient::new(grpc_channel.clone()),
    });

    handler.add_method_with_meta("trade_place_market_fak_buy", TradePlaceMarketFak {
        side: order::Side::Buy,
        rpc: TradeOrderRpcClient::new(grpc_channel.clone()),
    });

    handler.add_method_with_meta("trade_place_market_fak_sell", TradePlaceMarketFak {
        side: order::Side::Sell,
        rpc: TradeOrderRpcClient::new(grpc_channel.clone()),
    });

    handler.add_method_with_meta("trade_place_market_fok_buy", TradePlaceMarketFok {
        side: order::Side::Buy,
        rpc: TradeOrderRpcClient::new(grpc_channel.clone()),
    });

    handler.add_method_with_meta("trade_place_market_fok_sell", TradePlaceMarketFok {
        side: order::Side::Sell,
        rpc: TradeOrderRpcClient::new(grpc_channel.clone()),
    });

    handler.add_method_with_meta("trade_place_limit_buy", TradePlaceLimit {
        side: order::Side::Buy,
        rpc: TradeOrderRpcClient::new(grpc_channel.clone()),
    });

    handler.add_method_with_meta("trade_place_limit_sell", TradePlaceLimit {
        side: order::Side::Sell,
        rpc: TradeOrderRpcClient::new(grpc_channel.clone()),
    });

    handler.add_method_with_meta("trade_place_limit_fak_buy", TradePlaceLimitFak {
        side: order::Side::Buy,
        rpc: TradeOrderRpcClient::new(grpc_channel.clone()),
    });

    handler.add_method_with_meta("trade_place_limit_fak_sell", TradePlaceLimitFak {
        side: order::Side::Sell,
        rpc: TradeOrderRpcClient::new(grpc_channel.clone()),
    });

    handler.add_method_with_meta("trade_place_stop_buy", TradePlaceStop {
        side: order::Side::Buy,
        rpc: TradeOrderRpcClient::new(grpc_channel.clone()),
    });

    handler.add_method_with_meta("trade_place_stop_sell", TradePlaceStop {
        side: order::Side::Sell,
        rpc: TradeOrderRpcClient::new(grpc_channel.clone()),
    });

    handler.add_method_with_meta("trade_cancel_order", TradeCancelOrder {
        rpc: TradeOrderRpcClient::new(grpc_channel.clone()),
    });

    handler.add_method_with_meta("trade_search_order", TradeSearchOrder {
        rpc: TradeOrderRpcClient::new(grpc_channel.clone()),
    });

    handler.add_method_with_meta("trade_get_ticks", TradeGetTicks { });

    handler
}