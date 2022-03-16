# index-daemon (ICEX)

## Config

You can use _env vars_ or _config file_. You can specify _config file_ in _CLI params_ (described below). If you specify _config file_ - configs are taken from it. Else - from _env vars_. If you don't specify _config file_ and you don't have configs in _env vars_ - default configs are used.

All configs are optional.

### CLI params

You can get cli params description by calling program with "-h" cli param.

- **service_config** - path to service config file. Supports _yaml_ and _toml_
- **market_config** - path to market config file. Supports _yaml_ and _toml_
- **fill_historical** - fill historical data. Params: timestamp (contains comma-separated "from" and "to", "to" is optional), coins (uppercase comma-separated)

### Configs

#### service_config

- **log_level** - string. Variants: off, error, warn, info, debug, trace. Default: trace.
- **rest_timeout_sec** - u64. Timeout in seconds between requests to REST API.
- **ws** - string ("1" - on, default - off). Turn on websocket server.
- **ws_host** - string (default: 127.0.0.1). Websocket server host.
- **ws_port** - string (default: 8080). Websocket server port.
- **ws_answer_timeout_ms** - u64 (min - 100, default - 100). Timeout in ms between websocket answers.
- **historical** - string ("1" - on, default - off). Turn on historical data storage.
- **storage** - string. Variants: sled. Default: sled.

#### market_config

- **exchanges** - array (contains market names in _lowercase_. E.g. "binance")
- **coins** - array (contains coin name abbreviations in _uppercase_. E.g. "BTC")
- **index_coins** - coins that are used in index calculation. Same format as _coins_.
- **channels** - array. Variants: ticker, trades, book.

### Env vars format description

- env var name must be uppercase
- env var name is a concatenation of
  - prefix "APP"
  - _config group name_ (e.g., "service_config")
  - _config name_ (e.g., "log_level"), separated by a double underscore "__"
- env var name example: "APP__SERVICE_CONFIG__LOG_LEVEL"
- if param is array, env var value must contain string with comma-separated array values, e.g., "1,2,3"

## Supported exchanges

<table>
<tr>
<td>binance</td>
<td>bitfinex</td>
<td>bybit</td>
<td>coinbase</td>
<td>ftx</td>
<td>gateio</td>
<td>gemini</td>
</tr>
<tr>
<td>hitbtc</td>
<td>huobi</td>
<td>kraken</td>
<td>kucoin</td>
<td>okcoin</td>
<td>poloniex</td>
</tr>
</table>

## Websocket server

Websocket server configs are described above (section _Configs -> service_config -> ws_)

### Channels

#### available_coins (_not a channel, but a request_)

request json example:

```json
{
  "id": "some_id",
  "jsonrpc": "2.0",
  "method": "available_coins",
  "params": {}
}
```

#### index_price

subscription request json example:

```json
{
  "id": "some_id",
  "jsonrpc": "2.0",
  "method": "index_price",
  "params": {
    "coins": ["BTC", "ETH"],
    "frequency_ms": 100
  }
}
```

#### coin_average_price

subscription request json example:

```json
{
  "id": "some_id",
  "jsonrpc": "2.0",
  "method": "coin_average_price",
  "params": {
    "coins": ["BTC", "ETH"],
    "frequency_ms": 100
  }
}
```

#### coin_exchange_price

subscription request json example:

```json
{
  "id": "some_id",
  "jsonrpc": "2.0",
  "method": "coin_exchange_price",
  "params": {
    "coins": ["BTC", "ETH"],
    "exchanges": ["binance", "coinbase"],
    "frequency_ms": 100
  }
}
```

#### coin_exchange_volume

subscription request json example:

```json
{
  "id": "some_id",
  "jsonrpc": "2.0",
  "method": "coin_exchange_volume",
  "params": {
    "coins": ["BTC", "ETH"],
    "exchanges": ["binance", "coinbase"],
    "frequency_ms": 100
  }
}
```

#### coin_average_price_historical (_not a channel, but a request_)

- **interval** - interval between snapshots. Variants: second, minute, hour, day, week, month.
- **from** - timestamp from
- **to** - timestamp to

request json example:

```json
{
  "id": "some_id",
  "jsonrpc": "2.0",
  "method": "coin_average_price_historical",
  "params": {
    "coin": "BTC",
    "interval": "day",
    "from": 1643835600,
    "to": 1644440400
  }
}
```

#### index_price_candles

subscription request json example:

```json
{
  "id": "some_id",
  "jsonrpc": "2.0",
  "method": "index_price_candles",
  "params": {
    "frequency_ms": 50,
    "interval": "day"
  }
}
```

#### coin_average_price_candles

subscription request json example:

```json
{
  "id": "some_id",
  "jsonrpc": "2.0",
  "method": "coin_average_price_candles",
  "params": {
    "coins": ["BTC", "ETH"],
    "frequency_ms": 50,
    "interval": "day"
  }
}
```

#### coin_average_price_candles_historical (_not a channel, but a request_)

- **interval** - interval between snapshots. Variants: second, minute, hour, day, week, month.
- **from** - timestamp from
- **to** - timestamp to

request json example:

```json
{
  "id": "some_id",
  "jsonrpc": "2.0",
  "method": "coin_average_price_candles_historical",
  "params": {
    "coin": "BTC",
    "interval": "day",
    "from": 1643662800,
    "to": 1644872400
  }
}
```

#### unsubscribe (_not a channel, but a request_)

request json example:

```json
{
  "id": null,
  "jsonrpc": "2.0",
  "method": "unsubscribe",
  "params": {
    "method": "coin_exchange_price"
  }
}
```

### Description

- There can be only one subscription per channel. If you subscribe twice, then, previous subscription is declined and new subscription is activated.
- `id` must be unique or `null`.
- `id` of `unsubscribe` request is ignored.

## Note

There's only one fiat currency supported - `USD`, and it's hardcoded.
