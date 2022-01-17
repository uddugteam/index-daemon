# index-daemon (ICEX)

## Config

You can use _env vars_ or _config file_. You can specify _config file_ in _CLI params_ (described below). If you specify _config file_ - configs are taken from it. Else - from _env vars_. If you don't specify _config file_ and you don't have configs in _env vars_ - default configs are used.

All configs are optional.

### CLI params

- **service_config** - path to service config file. Supports _yaml_ and _toml_
- **market_config** - path to market config file. Supports _yaml_ and _toml_

### Configs

#### service_config

- **log_level** - string. Variants: error, warn, info, debug, trace.
- **rest_timeout_sec** - u64. Timeout in seconds between requests to REST API.
- **ws** - string ("1" - on, default - off). Turn on websocket sever.
- **ws_host** - string (default: 127.0.0.1). Websocket server host.
- **ws_port** - string (default: 8080). Websocket server port.

#### market_config

- **exchanges** - array (contains market names in _lowercase_. E.g. "binance")
- **coins** - array (contains coin name abbreviations in _uppercase_. E.g. "BTC")
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

## Note

There's only one fiat currency supported - "USD", and it's hardcoded.
