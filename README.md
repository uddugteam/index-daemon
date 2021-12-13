# index-daemon (ICEX)

## Config

You can use _env vars_ or _config file_. You can specify _config file_ in _CLI params_ (described below). If you specify _config file_ - configs are taken from it. Else - from _env vars_. If you don't specify _config file_ and you don't have configs in _env vars_ - default configs are used.

### CLI params

- **market_config** _(optional)_ - path to market config file. Supports _yaml_ and _toml_

### Configs

#### market_config

- **markets** - array (contains market names in _lowercase_. E.g. "binance")
- **coins** - array (contains coin name abbreviations in _uppercase_. E.g. "BTC")

### Env vars format description

- env var name must be uppercase
- env var name must have prefix "APP_"
- env var name is a concatenation of _config group name_ (e.g. "market_config") and _config name_ (e.g. "markets"), separated by underscore "_". E.g. "APP_MARKET_CONFIG_MARKETS"
- if param is array, env var value must contain string with comma-separated array values. E.g. "1,2,3"

## Note

There's only one fiat currency supported - "USD", and it's hardcoded.
