HTTP based API
===

状態を取得したいオブジェクトの種類に応じて使用するAPIが変わります。

1. [get transaction status](#get_transaction_status)
2. [get address utxos](#get_address_utxos)

## get transaction status

リクエストされたトランザクションの状態を取得します。
取得できるトランザクションの状態については、Responseをご参照ください。

#### Note

このAPIは5000ブロック(約１ヶ月分)を超えるブロックを監視対象としません。
また、指定されたトランザクションIDに不正なものが含まれていたとしても、それに関するエラーは報告されません。

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
      },
      "mined_block": {
        "description" "A hash of block which contains the transaction",
        "type": "string"
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
    "mined_block": "0000000000000000fakeblockhash1"
  },
  {
    "txid": "faketransactionid2",
    "confirmation": 8,
    "mined_block": "0000000000000000fakeblockhash2"
  }
]
```


## get address utxos

指定されたアドレスのUTXOを取得します。
UTXOの合計が、アドレスの残高となります。
取得できるUTXOの状態については、Responseをご参照ください。

#### Note

このAPIは5000ブロック(約１ヶ月分)を超えるブロックを監視対象としません。
また、指定されたアドレスに不正なものが含まれていたとしても、それに関するエラーは報告されません。

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
              "description": "Index where transaction output is contained",
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
