# webapi仕様書

## ブロック情報取得API [POST]

### 処理概要  

+ RPCcontent:
+ two endpoint 
```
   notification
    --> data sent to Server
    <-- data sent to Client
 -->
      {
        "jsonrpc": "2.0",
          "method": "notification",
          "params": {
            "taransation_id1":"1LeWWL94L3Nu7vf7mJvUfETnFpgMD8rs7U",
            "taransation_id2":"1LeWWL94L3Nu7vf7mJvUfETnFpgMD8rs7U",
            "taransation_id3":"1LeWWL94L3Nu7vf7mJvUfETnFpgMD8rs7U",
            "taransation_id4":"1LeWWL94L3Nu7vf7mJvUfETnFpgMD8rs7U"
          },
          "id": 1
      }
 <--
      {
        "jsonrpc": "2.0",
          "result":{
            "confirmation_id1":"2",
            "confirmation_id2":"6",
            "confirmation_id3":"3",
            "confirmation_id4":"2"
          },
          "params": {
            "taransation_id1":"1LeWWL94L3Nu7vf7mJvUfETnFpgMD8rs7U",
            "taransation_id2":"1LeWWL94L3Nu7vf7mJvUfETnFpgMD8rs7U",
            "taransation_id3":"1LeWWL94L3Nu7vf7mJvUfETnFpgMD8rs7U",
            "taransation_id4":"1LeWWL94L3Nu7vf7mJvUfETnFpgMD8rs7U"
          },
          "id": 2
      }

   result of UTXO
    --> data sent to Server
    <-- data sent to Client
 -->
      {
        "jsonrpc": "2.0",
          "method": "sum",
          "params": "transaction_id":"1LeWWL94L3Nu7vf7mJvUfETnFpgMD8rs7U",
          "id": 1
      }
 <--
      {
        "jsonrpc": "2.0",
          "result": "sumOfUtxo":"0.000100BTC",
          "params": "transaction_id":"1LeWWL94L3Nu7vf7mJvUfETnFpgMD8rs7U",
          "id": 2
      }
```
+ no content
+ push confirmation 
+ 125kb
+ get transaciton apo と つなぎ込みの部分
 
+ endpoint
+ transactionの状態の取得
+ utxoの取得
