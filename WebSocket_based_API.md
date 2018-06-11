WebSocket based API
===

## observe transaction status

トランザクションの状態を監視します。

### Client to Server

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

## observe address utxo

アドレスの残高を監視します。

### Client to Server

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
