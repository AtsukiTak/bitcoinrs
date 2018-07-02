Bitcoin network observing API
===

## TOC

- [Abstraction](#Abstraction)
- [HTTP based API](#HTTP_based_API)
- [WebSocket based API](#WebSocket_based_API)

## Abstraction
2種類のプロトコルを提供します。
1つはシンプルなHTTPベースのAPI、もう1つはWebSocketを利用したAPIです。
双方とも全てのパラメータはJsonオブジェクトにて渡されます。
以下に簡単に双方のメリット、デメリットを記載します。

### Abstraction of HTTP based API

Request-Response形式の、完全にステートレスなAPIを提供します。
クライアントは、現在の状態を取得したい全てのオブジェクト（アドレスの残高や、トランザクションのコンファメーションなど）を毎回のリクエストに含めます。
よって通信量は大きくなりますが、シンプルなAPIのためスケールや監視対象の管理が容易になり、また柔軟になります。
通信料が大きくなるとはいえ、各オブジェクトのサイズは64byteであるため、1万オブジェクトをリクエストしたときのサイズは高々640kbyteです。
厳密なリアルタイム性が求められない限りはこちらのAPIをお勧めします。

#### Merit

- 完全なステートレス
- スケールの容易性
- 監視対象の管理の容易性、柔軟性

#### Demerit

- リアルタイム性の欠如
- ネットワーク通信量

### Abstraction of WebSocket based API

Push通知形式の、コネクションレベルでステートフルなAPIを提供します。
クライアントは監視したいオブジェクトを適時追加します。
サーバは監視対象のオブジェクトに変更が加えられた場合にクライアントに通知を行います。
一度監視対象となったオブジェクトは、一定期間が過ぎた場合にのみ監視対象から外れます。
コネクションレベルでステートを持つため監視対象の管理が柔軟にできなくなりますが、リアルタイム性が担保されます。
リアルタイム性が強く求められる場合にのみ、こちらのAPIをお勧めします。

#### Merit

- リアルタイムでの応答
- ネットワーク通信量

#### Demerit

- コネクションレベルでステートフル
- 監視対象の管理の柔軟性

## HTTP based API

[here](./HTTP_based_API.md)

## WebSocket based API

[here](./WebSocket_based_API.md)
