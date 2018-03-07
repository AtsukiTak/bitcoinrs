# webapi仕様書

## ブロック情報取得API [POST]

### 処理概要  

+ RPCcontent:
```
   realtime notification
    --> data sent to Server
    <-- data sent to Client
 -->
      {
        "jsonrpc": "2.0",
          "method": "notification",
          "params": {"BitcoinAddress";"1LeWWL94L3Nu7vf7mJvUfETnFpgMD8rs7U"},
          "id": 1
      }
 <--
      {
        "jsonrpc": "2.0",
          "result":{ "confirmation":[1,2,3,4,5,6] } ,
          "params": {"BitcoinAddress";"1LeWWL94L3Nu7vf7mJvUfETnFpgMD8rs7U"},
          "id": 2
      }

   result of UTXO
    --> data sent to Server
    <-- data sent to Client
 -->
      {
        "jsonrpc": "2.0",
          "method": "sum",
          "params": {"BitcoinAddress_kabukomu";"1LeWWL94L3Nu7vf7mJvUfETnFpgMD8rs7U",
                     "BitcoinAddress_target";" 	1F1tAaz5x1HUXrCNLbtMDqcw6o5GNn4xqX"},
          "id": 1
      }
 <--
      {
        "jsonrpc": "2.0",
          "result":{ "SumOfUtxo": 0.00001000  } ,
          "params": {"BitcoinAddress_kabukomu";"1LeWWL94L3Nu7vf7mJvUfETnFpgMD8rs7U",
                     "BitcoinAddress_target";" 	1F1tAaz5x1HUXrCNLbtMDqcw6o5GNn4xqX"},
          "id": 2
      }
```
