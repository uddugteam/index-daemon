# index-daemon (ICEX)

## Config

You can use _env vars_ or _config file_. You can specify _config file_ in _CLI params_ (described below). If you specify _config file_ - configs are taken from it. Else - from _env vars_. If you don't specify _config file_ and you don't have configs in _env vars_ - default configs are used.

### CLI params

- **service_config** _(optional)_ - path to service config file. Supports _yaml_ and _toml_
- **market_config** _(optional)_ - path to market config file. Supports _yaml_ and _toml_

### Configs

#### service_config

- **log_level** - string. Variants: error, warn, info, debug, trace.

#### market_config

- **exchanges** - array (contains market names in _lowercase_. E.g. "binance")
- **coins** - array (contains coin name abbreviations in _uppercase_. E.g. "BTC")

### Env vars format description

- env var name must be uppercase
- env var name is a concatenation of
  - prefix "APP"
  - _config group name_ (e.g., "service_config")
  - _config name_ (e.g., "log_level"), separated by a double underscore "__"
- env var name example: "APP__SERVICE_CONFIG__LOG_LEVEL"
- if param is array, env var value must contain string with comma-separated array values, e.g., "1,2,3"

## Supported exchanges

- binance
- bitfinex
- coinbase
- poloniex
- kraken
- huobi
- hitbtc

## Note

There's only one fiat currency supported - "USD", and it's hardcoded.
