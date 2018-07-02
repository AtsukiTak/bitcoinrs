WebSocket based API
===

監視したいオブジェクトの種類に応じて使用するAPIが変わります。

1. [observe transaction status](#observe_transaction_status)
2. [observe address utxo](#observe_address_utxo)

## observe transaction status

トランザクションの状態を監視します。
基本的なオブジェクトの構成はHTTP based APIと同じです。

### Client to Server

クライアントからサーバに監視対象の追加依頼をします。
一度に複数の監視対象を追加することができます。

```json
{
    "$schema": "http://json-schema.org/schema#",
    "title": "Add observed transaction",

    "type": "array",
    "items": {
      "description": "Bitcoin transaction id",
      "type": "string"
    }
}
```

#### Example

```Json
["faketransactionid1", "faketransactionid2"]
```

### Server to Client

サーバからクライアントに通知を行います。
一度に複数の通知が行われます。

```Json
{
  "$schema": "http://json-schema.org/schema#",
  "title": "Updated transaction status",

  "type": "array",
  "items": {
    "title": "Tx Status",
    "type": "object",
    "properties": {
      "txid": {
        "type": "string"
      },
      "confirmation": {
        "description": "The number of blocks chained after the block.",
        "type": "number"
      },
      "mined_block": {
        "description": "A hash of block which contains the transaction",
        "typ": "string"
      }
    }
  }
}
```

#### Example

```Json
[
  {
    "txid": "faketransactionid1",
    "confirmation": 0,
    "mined_block": "0000000000000000000000fakeblockhash1"
  },
  {
    "txid": "faketransactionid2",
    "confirmation": 8,
    "mined_block": "0000000000000000000000fakeblockhash2"
  }
]
```

## observe address utxo

アドレスの残高を監視します。

### Client to Server

クライアントからサーバに監視対象の追加依頼をします。
一度に複数の監視対象を追加することができます。

```Json
{
  "$schema": "http://json-schema.org/schema#",
  "title": "Get Address Utxos Request",

  "type": "array",
  "items": {
    "description": "Bitcoin address",
    "type": "string"
  }
}
```

#### Example

```Json
["fakebitcoinaddress1", "fakebitcoinaddress2"]
```

### Server to Client

サーバからクライアントに通知を行います。
一度に複数の通知が行われます。

```Json
{
  "$schema": "http://json-schema.org/schema#",
  "title": "Get Address Utxos Response",

  "type": "array",
  "items": {
    "title": "address_utxos",
    "type": "object",
    "properties": {
      "address": {
        "type": "string"
      },
      "utxos": {
        "description": "array of utxo",
        "type": "array",
        "items": {
          "title": "utxo_object",
          "type": "object",
          "properties": {
            "txid": {
              "description": "Bitcoin transaction id",
              "type": "string"
            },
            "index": {
              "description": "Index at where transaction output is contained",
              "type": "number"
            },
            "amount": {
              "description": "Amount of btc",
              "type": "string"
            },
            "confirmation": {
              "type": "number"
            },
            "mined_block": {
              "type": "string"
            }
          }
        }
      }
    }
  }
}
```

#### Example

```Json
[
  {
    "address": "fakebitcoinaddress1",
    "utxos": [
      {
        "txid": "faketransactionid1",
        "index": 1,
        "amount": "0.00042",
        "confirmation": 2,
        "mined_block": "00000000000fakeblockhash1"
      },
      {
        "txid": "faketransactionid2",
        "index": 0,
        "amount": "0.0000003",
        "confirmation": 42,
        "mined_block": "00000000000fakeblockhash2"
      }
    ]
  },
  {
    "address": "fakebitcoinaddress2",
    "utxos": []
  }
]
```
