Bitcoin network observing API
===

## TOC

- [Abstraction](#Abstraction)
- [HTTP based API](#HTTP_based_API)
- [WebSocket based API](#WebSocket_based_API)

## Abstraction
2種類のプロトコルを提案します。
1つはシンプルなHTTPベースのAPI、もう1つはWebSocketを利用したAPIです。
双方とも全てのパラメータはJsonオブジェクトにて渡されます。
以下に簡単に双方のメリット、デメリットを記載します。

### Abstraction of HTTP based API

Request-Response形式の、完全にステートレスなAPIを提供します。
クライアントは、現在の状態を取得したい全てのオブジェクト（アドレスの残高など）を毎回のリクエストに含める必要があります。
よって通信量は大きくなりますが、シンプルなAPIのためスケールや管理が容易になります。
例えば、特定のアドレスを現在監視しているかどうかなどの管理は不要になります。
厳密なリアルタイム性が求められない限りはこちらのAPIをお勧めします。

#### Merit

- 完全なステートレス
- スケールの容易性

#### Demerit

- リアルタイム性の欠如
- ネットワーク通信量

### Abstraction of WebSocket based API

Push通知形式の、コネクションレベルでステートフルなAPIを提供します。
クライアントはコネクションの初期化時に監視したいオブジェクトの情報をサーバに通知します。
また、追加で監視したいオブジェクトは適時追加します。
サーバは監視対象のオブジェクトに変更が加えられた場合に、クライアントに通知を行います。
コネクションレベルでステートを持つため、ステートの管理が複雑になりますが、ある程度のリアルタイム性が担保されます。
複雑さやステートフルネスは大きなネックポイントになるため、リアルタイム性が強く求められる場合にのみ、こちらのAPIをお勧めします。

#### Merit

- リアルタイムでの応答
- ネットワーク通信量

#### Demerit

- コネクションレベルでステートフル
- 状態管理の複雑化

## HTTP based API

[here](./HTTP_based_API.md)

## WebSocket based API

[here](./WebSocket_based_API.md)
