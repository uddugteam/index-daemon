pub const MARKETS: [&str; 13] = [
    "binance", "bitfinex", "coinbase", "poloniex", "kraken", "huobi", "hitbtc", "okcoin", "gemini",
    "bybit", "gateio", "kucoin", "ftx",
];
pub const FIATS: [&str; 1] = ["USD"];
pub const COINS: [&str; 2] = ["BTC", "ETH"];
// pub const COINS: [&str; 50] = [
//     "BTC", "ETH", "BNB", "SOL", "USDC", "XRP", "DOT", "DOGE", "SHIB", "MATIC", "CRO", "WBTC",
//     "LTC", "UNI", "LINK", "DAI", "BCH", "TRX", "AXS", "MANA", "ATOM", "FTT", "SAND", "FIL", "ETC",
//     "XTZ", "CAKE", "XMR", "GALA", "GRT", "AAVE", "EOS", "LRC", "BTT", "MKR", "ENJ", "CRV", "ZEC",
//     "AMP", "NEO", "BAT", "KCS", "OKB", "CHZ", "DASH", "COMP", "NEXO", "XEM", "TUSD", "YFI",
// ];
pub const WS_SERVER_ALL_CHANNELS: [&str; 4] = [
    "coin_average_price",
    "coin_exchange_price",
    "coin_exchange_volume",
    "coin_average_price_candles",
];

pub const POLONIEX_EXCHANGE_PAIRS: [(&str, (&str, &str)); 456] = [
    ("336", ("BNB", "BTC")),
    ("462", ("BTC", "AAVE")),
    ("438", ("BTC", "AKRO")),
    ("451", ("BTC", "AMP")),
    ("177", ("BTC", "ARDR")),
    ("253", ("BTC", "ATOM")),
    ("324", ("BTC", "AVA")),
    ("210", ("BTC", "BAT")),
    ("475", ("BTC", "BCH")),
    ("464", ("BTC", "BCHA")),
    ("238", ("BTC", "BCHSV")),
    ("468", ("BTC", "BID")),
    ("232", ("BTC", "BNT")),
    ("14", ("BTC", "BTS")),
    ("333", ("BTC", "CHR")),
    ("194", ("BTC", "CVC")),
    ("24", ("BTC", "DASH")),
    ("162", ("BTC", "DCR")),
    ("27", ("BTC", "DOGE")),
    ("496", ("BTC", "DOT")),
    ("201", ("BTC", "EOS")),
    ("171", ("BTC", "ETC")),
    ("148", ("BTC", "ETH")),
    ("266", ("BTC", "ETHBNT")),
    ("382", ("BTC", "EXE")),
    ("494", ("BTC", "FARM")),
    ("414", ("BTC", "FCT2")),
    ("544", ("BTC", "FIL")),
    ("246", ("BTC", "FOAM")),
    ("489", ("BTC", "FRONT")),
    ("432", ("BTC", "FUND")),
    ("317", ("BTC", "FXC")),
    ("198", ("BTC", "GAS")),
    ("482", ("BTC", "GLM")),
    ("436", ("BTC", "HGET")),
    ("473", ("BTC", "INJ")),
    ("553", ("BTC", "KLV")),
    ("207", ("BTC", "KNC")),
    ("351", ("BTC", "LEND")),
    ("275", ("BTC", "LINK")),
    ("213", ("BTC", "LOOM")),
    ("250", ("BTC", "LPT")),
    ("355", ("BTC", "LRC")),
    ("163", ("BTC", "LSK")),
    ("50", ("BTC", "LTC")),
    ("229", ("BTC", "MANA")),
    ("295", ("BTC", "MATIC")),
    ("342", ("BTC", "MDT")),
    ("302", ("BTC", "MKR")),
    ("309", ("BTC", "NEO")),
    ("248", ("BTC", "NMR")),
    ("69", ("BTC", "NXT")),
    ("196", ("BTC", "OMG")),
    ("249", ("BTC", "POLY")),
    ("221", ("BTC", "QTUM")),
    ("353", ("BTC", "REN")),
    ("445", ("BTC", "REPV2")),
    ("454", ("BTC", "SAND")),
    ("150", ("BTC", "SC")),
    ("478", ("BTC", "SENSO")),
    ("204", ("BTC", "SNT")),
    ("290", ("BTC", "SNX")),
    ("690", ("BTC", "SOL")),
    ("630", ("BTC", "SRM")),
    ("168", ("BTC", "STEEM")),
    ("200", ("BTC", "STORJ")),
    ("369", ("BTC", "STPT")),
    ("89", ("BTC", "STR")),
    ("379", ("BTC", "SWAP")),
    ("312", ("BTC", "SWFTC")),
    ("403", ("BTC", "SWINGBY")),
    ("364", ("BTC", "SXP")),
    ("523", ("BTC", "TRU")),
    ("263", ("BTC", "TRX")),
    ("514", ("BTC", "TUSD")),
    ("486", ("BTC", "WBTC")),
    ("359", ("BTC", "WRX")),
    ("112", ("BTC", "XEM")),
    ("114", ("BTC", "XMR")),
    ("117", ("BTC", "XRP")),
    ("277", ("BTC", "XTZ")),
    ("178", ("BTC", "ZEC")),
    ("192", ("BTC", "ZRX")),
    ("340", ("BUSD", "BNB")),
    ("341", ("BUSD", "BTC")),
    ("306", ("DAI", "BTC")),
    ("307", ("DAI", "ETH")),
    ("358", ("ETH", "BAL")),
    ("211", ("ETH", "BAT")),
    ("190", ("ETH", "BCH")),
    ("347", ("ETH", "COMP")),
    ("202", ("ETH", "EOS")),
    ("172", ("ETH", "ETC")),
    ("515", ("ETH", "TUSD")),
    ("179", ("ETH", "ZEC")),
    ("193", ("ETH", "ZRX")),
    ("284", ("PAX", "BTC")),
    ("285", ("PAX", "ETH")),
    ("453", ("TRX", "AMP")),
    ("326", ("TRX", "AVA")),
    ("597", ("TRX", "BABYDOGE")),
    ("339", ("TRX", "BNB")),
    ("538", ("TRX", "BRG")),
    ("271", ("TRX", "BTT")),
    ("335", ("TRX", "CHR")),
    ("267", ("TRX", "ETH")),
    ("431", ("TRX", "FUND")),
    ("319", ("TRX", "FXC")),
    ("316", ("TRX", "JST")),
    ("553", ("TRX", "KLV")),
    ("276", ("TRX", "LINK")),
    ("297", ("TRX", "MATIC")),
    ("344", ("TRX", "MDT")),
    ("311", ("TRX", "NEO")),
    ("569", ("TRX", "NFT")),
    ("422", ("TRX", "PEARL")),
    ("292", ("TRX", "SNX")),
    ("274", ("TRX", "STEEM")),
    ("371", ("TRX", "STPT")),
    ("498", ("TRX", "SUN")),
    ("314", ("TRX", "SWFTC")),
    ("405", ("TRX", "SWINGBY")),
    ("366", ("TRX", "SXP")),
    ("420", ("TRX", "TAI")),
    ("535", ("TRX", "VSP")),
    ("273", ("TRX", "WIN")),
    ("361", ("TRX", "WRX")),
    ("268", ("TRX", "XRP")),
    ("279", ("TRX", "XTZ")),
    ("527", ("TUSD", "BTC")),
    ("528", ("TUSD", "ETH")),
    ("600", ("TUSD", "TRU")),
    ("254", ("USDC", "ATOM")),
    ("477", ("USDC", "BCH")),
    ("239", ("USDC", "BCHSV")),
    ("224", ("USDC", "BTC")),
    ("627", ("USDC", "BTT")),
    ("608", ("USDC", "BNB")),
    ("256", ("USDC", "DASH")),
    ("243", ("USDC", "DOGE")),
    ("257", ("USDC", "EOS")),
    ("258", ("USDC", "ETC")),
    ("225", ("USDC", "ETH")),
    ("252", ("USDC", "GRIN")),
    ("244", ("USDC", "LTC")),
    ("609", ("USDC", "LINK")),
    ("625", ("USDC", "MANA")),
    ("624", ("USDC", "MATIC")),
    ("666", ("USDC", "SHIB")),
    ("691", ("USDC", "SOL")),
    ("628", ("USDC", "SRM")),
    ("242", ("USDC", "STR")),
    ("264", ("USDC", "TRX")),
    ("513", ("USDC", "TUSD")),
    ("226", ("USDC", "USDT")),
    ("626", ("USDC", "XEM")),
    ("241", ("USDC", "XMR")),
    ("240", ("USDC", "XRP")),
    ("245", ("USDC", "ZEC")),
    ("288", ("USDJ", "BTC")),
    ("323", ("USDJ", "BTT")),
    ("289", ("USDJ", "TRX")),
    ("463", ("USDT", "AAVE")),
    ("601", ("USDT", "ACH1")),
    ("516", ("USDT", "ADABEAR")),
    ("517", ("USDT", "ADABULL")),
    ("540", ("USDT", "ADD")),
    ("439", ("USDT", "ADEL")),
    ("623", ("USDT", "AGLD")),
    ("550", ("USDT", "AKITA")),
    ("437", ("USDT", "AKRO")),
    ("604", ("USDT", "ALICE")),
    ("596", ("USDT", "ALPACA")),
    ("529", ("USDT", "ALPHA")),
    ("452", ("USDT", "AMP")),
    ("701", ("USDT", "ANGLE")),
    ("423", ("USDT", "ANK")),
    ("682", ("USDT", "ANY")),
    ("491", ("USDT", "API3")),
    ("255", ("USDT", "ATOM")),
    ("588", ("USDT", "AUTO")),
    ("606", ("USDT", "AUDIO")),
    ("325", ("USDT", "AVA")),
    ("593", ("USDT", "AXS")),
    ("551", ("USDT", "B20")),
    ("504", ("USDT", "BAC")),
    ("493", ("USDT", "BADGER")),
    ("357", ("USDT", "BAL")),
    ("387", ("USDT", "BAND")),
    ("212", ("USDT", "BAT")),
    ("476", ("USDT", "BCH")),
    ("465", ("USDT", "BCHA")),
    ("298", ("USDT", "BCHBEAR")),
    ("299", ("USDT", "BCHBULL")),
    ("345", ("USDT", "BCHC")),
    ("259", ("USDT", "BCHSV")),
    ("320", ("USDT", "BCN")),
    ("539", ("USDT", "BDP")),
    ("280", ("USDT", "BEAR")),
    ("469", ("USDT", "BID")),
    ("636", ("USDT", "BIFI")),
    ("607", ("USDT", "BIT")),
    ("401", ("USDT", "BLY")),
    ("337", ("USDT", "BNB")),
    ("706", ("USDT", "BNX")),
    ("509", ("USDT", "BOND")),
    ("611", ("USDT", "BP")),
    ("457", ("USDT", "BREE")),
    ("537", ("USDT", "BRG")),
    ("293", ("USDT", "BSVBEAR")),
    ("294", ("USDT", "BSVBULL")),
    ("121", ("USDT", "BTC")),
    ("542", ("USDT", "BTCST")),
    ("637", ("USDT", "BTRST")),
    ("270", ("USDT", "BTT")),
    ("281", ("USDT", "BULL")),
    ("338", ("USDT", "BUSD")),
    ("587", ("USDT", "BURGER")),
    ("304", ("USDT", "BVOL")),
    ("363", ("USDT", "BZRX")),
    ("598", ("USDT", "C98")),
    ("584", ("USDT", "CAKE")),
    ("656", ("USDT", "CHESS")),
    ("334", ("USDT", "CHR")),
    ("646", ("USDT", "CHZ")),
    ("602", ("USDT", "CLV")),
    ("511", ("USDT", "COMBO")),
    ("346", ("USDT", "COMP")),
    ("620", ("USDT", "COOL")),
    ("427", ("USDT", "CORN")),
    ("433", ("USDT", "CREAM")),
    ("695", ("USDT", "CRO")),
    ("425", ("USDT", "CRT")),
    ("397", ("USDT", "CRV")),
    ("508", ("USDT", "CUDOS")),
    ("350", ("USDT", "CUSDT")),
    ("443", ("USDT", "CVP")),
    ("308", ("USDT", "DAI")),
    ("122", ("USDT", "DASH")),
    ("374", ("USDT", "DEC")),
    ("561", ("USDT", "DEGO")),
    ("395", ("USDT", "DEXT")),
    ("262", ("USDT", "DGB")),
    ("441", ("USDT", "DHT")),
    ("389", ("USDT", "DIA")),
    ("674", ("USDT", "DOE")),
    ("692", ("USDT", "DORA")),
    ("216", ("USDT", "DOGE")),
    ("388", ("USDT", "DOS")),
    ("407", ("USDT", "DOT")),
    ("616", ("USDT", "DFA")),
    ("630", ("USDT", "DYDX")),
    ("686", ("USDT", "DYP")),
    ("657", ("USDT", "EFI")),
    ("563", ("USDT", "ELON")),
    ("645", ("USDT", "ENJ")),
    ("203", ("USDT", "EOS")),
    ("330", ("USDT", "EOSBEAR")),
    ("329", ("USDT", "EOSBULL")),
    ("574", ("USDT", "eRSDL")),
    ("586", ("USDT", "EPS")),
    ("653", ("USDT", "ERN")),
    ("500", ("USDT", "ESD")),
    ("173", ("USDT", "ETC")),
    ("149", ("USDT", "ETH")),
    ("300", ("USDT", "ETHBEAR")),
    ("301", ("USDT", "ETHBULL")),
    ("383", ("USDT", "EXE")),
    ("495", ("USDT", "FARM")),
    ("413", ("USDT", "FCT2")),
    ("545", ("USDT", "FIL")),
    ("638", ("USDT", "FLOKI")),
    ("562", ("USDT", "FORTH")),
    ("683", ("USDT", "FREN")),
    ("490", ("USDT", "FRONT")),
    ("429", ("USDT", "FSW")),
    ("524", ("USDT", "FTT")),
    ("430", ("USDT", "FUND")),
    ("318", ("USDT", "FXC")),
    ("685", ("USDT", "FXS")),
    ("633", ("USDT", "GALA")),
    ("661", ("USDT", "GMEE")),
    ("385", ("USDT", "GEEQ")),
    ("444", ("USDT", "GHST")),
    ("483", ("USDT", "GLM")),
    ("618", ("USDT", "GLYPH")),
    ("708", ("USDT", "GODZ")),
    ("261", ("USDT", "GRIN")),
    ("497", ("USDT", "GRT")),
    ("578", ("USDT", "GTC")),
    ("484", ("USDT", "HEGIC")),
    ("684", ("USDT", "HEX")),
    ("435", ("USDT", "HGET")),
    ("305", ("USDT", "IBVOL")),
    ("655", ("USDT", "ICE")),
    ("639", ("USDT", "IDIA")),
    ("652", ("USDT", "ILV")),
    ("474", ("USDT", "INJ")),
    ("424", ("USDT", "JFI")),
    ("315", ("USDT", "JST")),
    ("541", ("USDT", "KCS")),
    ("688", ("USDT", "KEEP")),
    ("552", ("USDT", "KLV")),
    ("480", ("USDT", "KP3R")),
    ("377", ("USDT", "KTON")),
    ("575", ("USDT", "LEASH")),
    ("644", ("USDT", "LATTE")),
    ("634", ("USDT", "LDO")),
    ("352", ("USDT", "LEND")),
    ("322", ("USDT", "LINK")),
    ("332", ("USDT", "LINKBEAR")),
    ("331", ("USDT", "LINKBULL")),
    ("548", ("USDT", "LIVE")),
    ("505", ("USDT", "LON")),
    ("526", ("USDT", "LPT")),
    ("555", ("USDT", "LQTY")),
    ("356", ("USDT", "LRC")),
    ("218", ("USDT", "LSK")),
    ("123", ("USDT", "LTC")),
    ("518", ("USDT", "LTCBEAR")),
    ("519", ("USDT", "LTCBULL")),
    ("556", ("USDT", "LUSD")),
    ("614", ("USDT", "LUMI")),
    ("231", ("USDT", "MANA")),
    ("619", ("USDT", "MASK")),
    ("296", ("USDT", "MATIC")),
    ("610", ("USDT", "MBOX")),
    ("681", ("USDT", "MC")),
    ("396", ("USDT", "MCB")),
    ("343", ("USDT", "MDT")),
    ("442", ("USDT", "MEME")),
    ("448", ("USDT", "MEXP")),
    ("559", ("USDT", "MIR")),
    ("564", ("USDT", "MIST")),
    ("303", ("USDT", "MKR")),
    ("640", ("USDT", "MLN")),
    ("481", ("USDT", "MPH")),
    ("698", ("USDT", "MPL")),
    ("367", ("USDT", "MTA")),
    ("310", ("USDT", "NEO")),
    ("566", ("USDT", "MVL")),
    ("703", ("USDT", "NEXO")),
    ("568", ("USDT", "NFT")),
    ("589", ("USDT", "NRV")),
    ("488", ("USDT", "NU")),
    ("558", ("USDT", "NFTX")),
    ("400", ("USDT", "OCEAN")),
    ("648", ("USDT", "OGN")),
    ("579", ("USDT", "OKB")),
    ("399", ("USDT", "OM")),
    ("501", ("USDT", "ONEINCH")),
    ("402", ("USDT", "OPT")),
    ("665", ("USDT", "O3")),
    ("612", ("USDT", "OSK")),
    ("286", ("USDT", "PAX")),
    ("510", ("USDT", "PBTC35A")),
    ("421", ("USDT", "PEARL")),
    ("694", ("USDT", "PEOPLE")),
    ("599", ("USDT", "PERP")),
    ("392", ("USDT", "PERX")),
    ("580", ("USDT", "POL")),
    ("460", ("USDT", "POLS")),
    ("680", ("USDT", "POLYDOGE")),
    ("700", ("USDT", "PRINTS")),
    ("406", ("USDT", "PRQ")),
    ("702", ("USDT", "PSP")),
    ("617", ("USDT", "PUNK")),
    ("223", ("USDT", "QTUM")),
    ("566", ("USDT", "QUICK")),
    ("658", ("USDT", "RACA")),
    ("696", ("USDT", "RARE")),
    ("447", ("USDT", "RARI")),
    ("605", ("USDT", "RD")),
    ("502", ("USDT", "REEF")),
    ("354", ("USDT", "REN")),
    ("446", ("USDT", "REPV2")),
    ("662", ("USDT", "REVV")),
    ("456", ("USDT", "RFUEL")),
    ("378", ("USDT", "RING")),
    ("411", ("USDT", "RSR")),
    ("426", ("USDT", "SAL")),
    ("455", ("USDT", "SAND")),
    ("219", ("USDT", "SC")),
    ("479", ("USDT", "SENSO")),
    ("543", ("USDT", "SFI")),
    ("549", ("USDT", "SHIB")),
    ("647", ("USDT", "SLP")),
    ("291", ("USDT", "SNX")),
    ("689", ("USDT", "SOL")),
    ("654", ("USDT", "SPELL")),
    ("670", ("USDT", "SQUID")),
    ("525", ("USDT", "SRM")),
    ("707", ("USDT", "SSG")),
    ("362", ("USDT", "STAKE")),
    ("321", ("USDT", "STEEM")),
    ("370", ("USDT", "STPT")),
    ("125", ("USDT", "STR")),
    ("434", ("USDT", "SUN")),
    ("651", ("USDT", "SUPER")),
    ("415", ("USDT", "SUSHI")),
    ("380", ("USDT", "SWAP")),
    ("313", ("USDT", "SWFTC")),
    ("404", ("USDT", "SWINGBY")),
    ("428", ("USDT", "SWRV")),
    ("365", ("USDT", "SXP")),
    ("419", ("USDT", "TAI")),
    ("381", ("USDT", "TEND")),
    ("650", ("USDT", "TLM")),
    ("641", ("USDT", "TOKE")),
    ("384", ("USDT", "TRADE")),
    ("393", ("USDT", "TRB")),
    ("687", ("USDT", "TRIBE")),
    ("507", ("USDT", "TRU")),
    ("265", ("USDT", "TRX")),
    ("282", ("USDT", "TRXBEAR")),
    ("283", ("USDT", "TRXBULL")),
    ("512", ("USDT", "TUSD")),
    ("699", ("USDT", "UDT")),
    ("376", ("USDT", "UMA")),
    ("635", ("USDT", "UMB")),
    ("440", ("USDT", "UNI")),
    ("287", ("USDT", "USDJ")),
    ("560", ("USDT", "UST")),
    ("458", ("USDT", "VALUE")),
    ("534", ("USDT", "VSP")),
    ("487", ("USDT", "WBTC")),
    ("522", ("USDT", "WETH")),
    ("557", ("USDT", "WHALE")),
    ("272", ("USDT", "WIN")),
    ("642", ("USDT", "WNCG")),
    ("412", ("USDT", "WNXM")),
    ("643", ("USDT", "WOO")),
    ("360", ("USDT", "WRX")),
    ("632", ("USDT", "XCAD")),
    ("697", ("USDT", "XDEFI")),
    ("629", ("USDT", "XEM")),
    ("499", ("USDT", "XFLR")),
    ("520", ("USDT", "XLMBEAR")),
    ("521", ("USDT", "XLMBULL")),
    ("126", ("USDT", "XMR")),
    ("127", ("USDT", "XRP")),
    ("328", ("USDT", "XRPBEAR")),
    ("327", ("USDT", "XRPBULL")),
    ("278", ("USDT", "XTZ")),
    ("565", ("USDT", "XOR")),
    ("585", ("USDT", "XVS")),
    ("368", ("USDT", "YFI")),
    ("416", ("USDT", "YFII")),
    ("418", ("USDT", "YFL")),
    ("594", ("USDT", "YFX")),
    ("603", ("USDT", "YGG")),
    ("390", ("USDT", "ZAP")),
    ("180", ("USDT", "ZEC")),
    ("531", ("USDT", "ZKS")),
    ("485", ("USDT", "ZLOT")),
    ("220", ("USDT", "ZRX")),
];
