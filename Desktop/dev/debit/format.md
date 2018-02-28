
# webapi仕様書(JSONRPC)

## ブロック情報取得 [/v1/bitcoin/{?bitcoinaddress}] -- (1)
 
### ブロック情報取得API [POST]
 
#### 処理概要  -- (2)
 
* 指定したbitcoinaddressの情報を返す。
* bitcoinaddress必須。指定がない場合、BadRequest(Response 300)を返す。
 
+ Parameters  -- (3)
 
    + bitcoinaddress : 1BitQEtcoxAnViwUYX9k6KupmmsEfWrGnr (string, optional) - ビットコインアドレス 

+ Response 200 (application/json)

    + Attributes
          tx_hash   トランザクションのハッシュ
          block_height  このトランザクションを含むブロックの高さ
          inputs  Txin の配列
              prev_hash   この Txin を出力した前のトランザクションのハッシュ
              prev_index  前のトランザクションにおける Txout のインデックス
              value   ビットコインの数量（satoshi単位）
              script  署名スクリプト（16進数表記）
              address   ビットコインアドレス
              sequence  シーケンス番号
          outputs   Txoutの配列
              value   ビットコインの数量（satoshi単位）
              script  スクリプト（16進数表記）
              address   ビットコインアドレス

        {
          "tx_hash": "0562d1f063cd4127053d838b165630445af5e480ceb24e1fd9ecea52903cb772",
          "block_height": 370470,
          "received_date": "2015-08-19T00:46:01.017",
          "inputs": [
            {
              "prev_hash": "d43008e8019a641565615ec02b8c7d107fc470c4d500dcbb9299756afb9ee4b5",
              "prev_index": 1,
              "value": 350080000,
              "script": "473044022062c6c81df6726573297c2288b7ed7be1d85c5b79ccf7e79a5c808c0e13b5e4f202200f2ae49c1119b2d74239086d12ec3ce96e408fb24adffe0dbb2424fbd7d1da27014104e8c8c15f6b714b46ed7ee49c023c009de5ed5b98bd60ca2d762be235274761f1db206f85b0072cc3ff62f727492740bfe0879d6135fe63b63454fe8aab268e13",
              "address": "1ygL6TVoUiGXVGcjeGLee5zsarrbqYgwk",
              "sequence": 4294967295
            }
          ],
          "outputs": [
            {
              "value": 60000,
              "script": "76a914b066879afa8a2e35419a4826a6f8702438c5fc4388ac",
              "address": "1H5ikemcbo71w1JXw1J3J8aSuPHwABEHEH"
            },
            {
              "value": 350000000,
              "script": "76a914e84e80236d767cb7b4c344e3007af25ea7a5f12788ac",
              "address": "1NBKopSCvBSD5vrAamyrgQ7W9a9wLpo4Gx"
            }
          ]
        }
