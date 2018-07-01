HTTP based API
===

There are two kind of API

1. [get transaction status](#get_transaction_status)
2. [get address utxos](#get_address_utxos)

## get transaction status

リクエストされたトランザクションの状態を取得します。

トランザクションの状態は、以下の要素で構成されます。

- confirmation

#### Note

このAPIは5000ブロック(約１ヶ月分)を超えるブロックを監視対象としません。

### Request

#### Json schema

```Json
{
    "$schema": "http://json-schema.org/schema#",
    "title": "Get Transaction Status Request",

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

### Response

#### Json schema

```Json
{
  "$schema": "http://json-schema.org/schema#",
  "title": "Get Transaction Status Response",

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
    "confirmation": 0
  },
  {
    "txid": "faketransactionid2",
    "confirmation": 8
  }
]
```

#### Note

指定されたトランザクションIDに不正なものが含まれていたとしても、それに関するエラーは報告されません。

## get address utxos

指定されたアドレスのUTXOを取得します。

#### Note

このAPIは5000ブロック(約１ヶ月分)を超えるブロックを監視対象としません。

### Request

#### Json schema

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

### Response

#### Json schema

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
        "amount": "0.00042"
      },
      {
        "txid": "faketransactionid2",
        "index": 0,
        "amount": "0.0000003"
      }
    ]
  },
  {
    "address": "fakebitcoinaddress2",
    "utxos": []
  }
]
```

#### Note

指定されたアドレスに不正なものが含まれていたとしても、それに関するエラーは報告されません。
