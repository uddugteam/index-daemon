use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use xmltree::Element;

use crate::worker::market_helpers::market::{market_factory, Market};
use crate::worker::market_helpers::market_spine::MarketSpine;
use crate::worker::other_helpers::config_parser::ConfigParser;
use crate::worker::xml_reader::*;

pub const DEFAULT_CONFIG_PATH: &str = "./resources/curr_daemon.config";
pub const DEFAULT_CONFIG_PATH_2: &str = "/etc/curr_daemon/curr_daemon.config";
pub const XML_CONFIG_FILE_PATH: &str = "./resources/config.xml";

pub struct Worker {
    config_path: Option<String>,
    coins: HashMap<String, String>,
    fiats: HashMap<String, String>,
    markets: Vec<Arc<Mutex<dyn Market + Send>>>,
    arc: Option<Arc<Mutex<Self>>>,
}

impl Worker {
    pub fn new(config_path: Option<String>) -> Arc<Mutex<Self>> {
        let worker = Worker {
            config_path,
            coins: HashMap::new(),
            fiats: HashMap::new(),
            markets: Vec::new(),
            arc: None,
        };

        let worker = Arc::new(Mutex::new(worker));
        worker.lock().unwrap().set_arc(Arc::clone(&worker));

        worker
    }

    pub fn set_arc(&mut self, arc: Arc<Mutex<Self>>) {
        self.arc = Some(arc);
    }

    // TODO: Implement
    pub fn recalculate_total_volume(
        &self,
        currency: String,
        pairs: &HashMap<String, (String, String)>,
    ) {
        // println!("called Worker::recalculate_total_volume()");

        let mut volume_val: f64 = 0.0;
        let mut markets_count: i32 = 0;
        let mut buf: String = String::new();
        let mut success: bool = true;
        let mut fail_count: i32 = 0;

        for market in &self.markets {
            for pair in pairs {
                if pair.1 .0 == currency {
                    markets_count += 1;
                    let volume: f64 = market
                        .lock()
                        .unwrap()
                        .get_total_volume(&pair.1 .0, &pair.1 .1);

                    if volume.eq(&-1_f64) {
                        fail_count += 1;
                        success = false;

                        // C++: loggingHelper->printLog("volume", 2, currency + " " + market->getName() + " " + pair.first);
                        println!(
                            "{} {} {}",
                            currency,
                            market.lock().unwrap().get_spine().name,
                            pair.0
                        );
                    }

                    volume_val += volume;

                    buf = format!(
                        "{}: {}_{} {} ",
                        market.lock().unwrap().get_spine().name,
                        pair.1 .0,
                        pair.1 .1,
                        volume
                    );
                }
            }
        }

        if !success {
            // C++: loggingHelper->printLog("volume", 2, currency + " NOT ENOUGH VALUES " + std::to_string(markets_count - failCount) + " OUT OF " + std::to_string(markets_count));
            println!(
                "{} NOT ENOUGH VALUES {} OUT OF {}",
                currency,
                markets_count - fail_count,
                markets_count
            );

            return;
        }

        // C++: loggingHelper->printLog("volume", 1, currency + ": " + std::to_string(volume_val));
        // C++: loggingHelper->printLog("volume", 3, currency + ": " + buf);
        println!("{}: {}", currency, volume_val);
        println!("{}: {}", currency, buf);

        // C++: code, associated with Redis
        // C++: code, associated with candles
    }

    fn configure(&mut self) {
        let path: String = match &self.config_path {
            Some(config_path_str) if Path::new(config_path_str).exists() => {
                config_path_str.to_string()
            }
            _ if Path::new(DEFAULT_CONFIG_PATH).exists() => DEFAULT_CONFIG_PATH.to_string(),
            _ => DEFAULT_CONFIG_PATH_2.to_string(),
        };

        // C++: loggingHelper->printLog("default", 1, "MainConfig path = " + path);
        println!("MainConfig path = {}", path);

        let config_parser = ConfigParser::new(&path)
            .map_err(|err| panic!("Config file open/read error: {}.", err))
            .unwrap();

        let pq_enabled: bool = config_parser.get_param("postgres.enabled") == Some("1");
        // C++: postgresHelper->setEnabled(pq_enabled);
        if pq_enabled {
            // let pq_host = config_parser
            //     .get_param("postgres.host")
            //     .expect("Param postgres.host not found");
            // let pq_port = config_parser
            //     .get_param("postgres.port")
            //     .expect("Param postgres.port not found")
            //     .parse::<u32>()
            //     .expect("Param postgres.port parse error");
            // let pq_dbname = config_parser
            //     .get_param("postgres.dbname")
            //     .expect("Param postgres.dbname not found");
            // let pq_user = config_parser
            //     .get_param("postgres.username")
            //     .expect("Param postgres.username not found");
            // let pq_pass = config_parser
            //     .get_param("postgres.password")
            //     .expect("Param postgres.password not found");
            // let pq_max_conns = config_parser
            //     .get_param("postgres.maxConnections")
            //     .and_then(|v| v.parse::<u32>().ok());

            // C++: postgresHelper->setLoggingHelper(this->loggingHelper);
            // C++: postgresHelper->init(pqHost, pqPort, pqDBName, pqUser, pqPass, pqMaxConns);
        }

        let coins_path = config_parser
            .get_param("coins.config")
            .expect("Param coins.config not found");

        // C++: TiXmlDocument doc("../resources/config.xml");

        let xml_config_reader =
            match Element::parse(fs::read_to_string(XML_CONFIG_FILE_PATH).unwrap().as_bytes()) {
                Ok(xml_reader) => xml_reader,
                Err(err) => {
                    // C++: loggingHelper->printLog("default", 1, "Basic Config not Found. Checking alternative config.");
                    println!("Basic Config open error: {}.", err.to_string());
                    println!("Checking alternative config.");

                    match Element::parse(fs::read_to_string(coins_path).unwrap().as_bytes()) {
                        Ok(xml_reader) => xml_reader,
                        Err(err) => {
                            // C++: loggingHelper->printLog("default", 1, "Config not Found. Abort.");
                            println!("Config open error: {}.", err.to_string());
                            println!("Abort.");
                            panic!("NoConfig");
                        }
                    }
                }
            };

        // C++: loggingHelper->printLog("default", 1, "CoinsConfig path = " + coins_path);
        println!("CoinsConfig path = {}", coins_path);

        let custom_index = xml_config_reader.get_child("custom-index").unwrap();
        for child in custom_index.children.iter() {
            let index_name = child.as_element().unwrap().get_text().unwrap();

            // C++: calcIndex.emplace(indexName, getCustomIndexesSummary(indexName, curl));
            // C++: postgresHelper->dropToDB({"short, name"}, {"'" + indexName + "'", "'" + indexName + "'"}, "tickers", true);
            println!(
                "Called {} with params: table={}; keys={}; values={}",
                "postgresHelper->dropToDB()",
                "tickers",
                "short, name",
                index_name.clone() + ", " + index_name
            );
        }

        // C++: loggingHelper->printLog("general", 1, "CustomIndexes configured successfully.");
        // println!("CustomIndexes configured successfully.");

        let markets_global = xml_config_reader.get_child("markets-global").unwrap();
        let update_ticker: bool = get_el_child_text(markets_global, "update-ticker") == "1";
        let update_last_trade: bool = get_el_child_text(markets_global, "update-last-trade") == "1";
        let update_depth: bool = get_el_child_text(markets_global, "update-depth") == "1";
        let fiat_refresh_time: u64 =
            get_el_child_text_as(markets_global, "fiat-refresh-time").unwrap();

        // C++: loggingHelper->printLog("general", 1, "GlobalMarketsParams configured successfully.");
        // println!("GlobalMarketsParams configured successfully.");

        // C++: pingPong.init(std::stoi(config_parser.get_param("network.pingPort")));
        // C++: pingPong.setLoggingHelper(loggingHelper);

        // C++: loggingHelper->printLog("general", 1, "Ping-Pong configured successfully.");
        // println!("Ping-Pong configured successfully.");

        // C++: code, associated with Redis
        // C++: code, associated with candles

        // C++: loggingHelper->printLog("general", 1, "RedisHelper configured successfully.");
        // println!("RedisHelper configured successfully.");

        println!("Get coins from xml BEGIN.");
        let coins = xml_config_reader.get_child("coins").unwrap();
        for coin in coins.children.iter() {
            let short: String = get_node_child_text_as(coin, "short-name").unwrap();
            let full: String = get_node_child_text_as(coin, "full-name").unwrap();

            self.coins.insert(short.clone(), full.clone());

            // C++: postgresHelper->dropToDB({"short, name"}, {"'" + shrt + "'", "'" + full + "'"}, "tickers", true);
            println!(
                "Called {} with params: table={}; keys={}; values={}",
                "postgresHelper->dropToDB()",
                "tickers",
                "short, name",
                short + ", " + &full
            );
        }
        println!("Get coins from xml END.");

        println!("Get fiats from xml BEGIN.");
        let fiats = xml_config_reader.get_child("fiats").unwrap();
        for fiat in fiats.children.iter() {
            let short: String = get_node_child_text_as(fiat, "short").unwrap();
            let full: String = get_node_child_text_as(fiat, "name").unwrap();

            self.fiats.insert(short.clone(), full.clone());

            // C++: postgresHelper->dropToDB({"short, name"}, {"'" + shrt + "'", "'" + full + "'"}, "tickers", true);
            println!(
                "Called {} with params: table={}; keys={}; values={}",
                "postgresHelper->dropToDB()",
                "tickers",
                "short, name",
                short + ", " + &full
            );
        }
        println!("Get fiats from xml END.");

        // C++: loggingHelper->printLog("general", 1, "CoinsArray configured successfully.");
        println!("CoinsArray configured successfully.");

        // C++: postgresHelper->dropToDB({"name"}, {"'vol'"}, "data_types", true);
        // C++: postgresHelper->dropToDB({"name"}, {"'cap'"}, "data_types", true);
        // C++: postgresHelper->dropToDB({"name"}, {"'price'"}, "data_types", true);
        println!(
            "Called {} with params: table={}; keys={}; values={}",
            "postgresHelper->dropToDB()", "data_types", "name", "vol"
        );
        println!(
            "Called {} with params: table={}; keys={}; values={}",
            "postgresHelper->dropToDB()", "data_types", "name", "cap"
        );
        println!(
            "Called {} with params: table={}; keys={}; values={}",
            "postgresHelper->dropToDB()", "data_types", "name", "price"
        );

        println!("Get entities from xml BEGIN.");
        let entities = xml_config_reader.get_child("entities").unwrap();
        for entity in entities.children.iter() {
            let status: String = get_node_child_text_as(entity, "status").unwrap();
            let name: String = get_node_child_text_as(entity, "name").unwrap();
            let api_url: String = get_node_child_text_as(entity, "api-url").unwrap();
            let error_message: String = get_node_child_text_as(entity, "error-message").unwrap();
            let delay: u32 = get_node_child_text_as(entity, "delay").unwrap();

            let worker_2 = Arc::clone(self.arc.as_ref().unwrap());

            let market_spine = MarketSpine::new(
                worker_2,
                status.parse().unwrap_or_else(|_| {
                    panic!(
                        "Parse status error. Market: {}. Status not found: {}",
                        name, status
                    )
                }),
                name,
                api_url,
                error_message,
                delay,
                update_ticker,
                update_last_trade,
                update_depth,
                fiat_refresh_time,
            );

            let market = market_factory(market_spine);

            let currency_mask_pairs = entity
                .as_element()
                .unwrap()
                .get_child("currency_mask_pairs")
                .unwrap();
            for child in currency_mask_pairs.children.iter() {
                let pair_string = child.as_element().unwrap().get_text().unwrap().to_string();
                let parts: Vec<&str> = pair_string.split(':').collect();
                market
                    .lock()
                    .unwrap()
                    .get_spine_mut()
                    .add_mask_pair((parts[0], parts[1]));
            }

            // C++: ThreadPool *threadPool = new ThreadPool(30);

            // C++: postgresHelper->dropToDB({"name"}, {"'" + market->getName() + "'"}, "exchanges", true);
            println!(
                "Called {} with params: table={}; keys={}; values={}",
                "postgresHelper->dropToDB()",
                "exchanges",
                "name",
                market.lock().unwrap().get_spine().name
            );

            println!("Get exchange_pairs from xml BEGIN.");
            let exchange_pairs = entity
                .as_element()
                .unwrap()
                .get_child("exchange_pairs")
                .unwrap();
            for exchange_pair in exchange_pairs.children.iter() {
                let pair_string = get_node_child_text(exchange_pair, "value");
                let parts: Vec<&str> = pair_string.split(':').collect();

                if !self.fiats.contains_key(parts[0]) && !self.coins.contains_key(parts[0]) {
                    // C++: postgresHelper->dropToDB({"short, name"}, {"'" + a + "'", "'" + a + "'"}, "tickers", true);
                    println!(
                        "Called {} with params: table={}; keys={}; values={}",
                        "postgresHelper->dropToDB()",
                        "tickers",
                        "short, name",
                        parts[0].to_string() + ", " + parts[0]
                    );
                }

                if !self.fiats.contains_key(parts[1]) && !self.coins.contains_key(parts[1]) {
                    // C++: postgresHelper->dropToDB({"short, name"}, {"'" + b + "'", "'" + b + "'"}, "tickers", true);
                    println!(
                        "Called {} with params: table={}; keys={}; values={}",
                        "postgresHelper->dropToDB()",
                        "tickers",
                        "short, name",
                        parts[1].to_string() + ", " + parts[1]
                    );
                }

                let conversion: String = get_node_child_text(exchange_pair, "conversion");
                market
                    .lock()
                    .unwrap()
                    .add_exchange_pair((parts[0], parts[1]), &conversion);

                // C++: if (postgresHelper->isEnabled()) {
                // C++: threadPool->runAsync([postgresHelper, a, b, market] {
                // C++: block of code, associated with asynchronous queries to postgres
                // C++: }); // end threadPool->runAsync()
                // C++: } // endif
            }
            println!("Get exchange_pairs from xml END.");

            // C++: free(threadPool);

            // C++: market->setCoinsArrayRef(&coins);
            // C++: market->setAddOrUpdateIndexHandler(this->addOrUpdateIndex);
            // C++: market->setReCalculateIndexHandler(this->reCalculate);

            // C++: market->setGetIndexHandler(this->getValue);
            // C++: market->setAddRedisKeyHandler(this->addRedisKey);
            // C++: market->setRequestPool(&pool);
            // C++: market->setLoggingHelper(loggingHelper);
            // C++: market->setUpdateHashMarketDataHandler(this->updateHashForMarket);
            // C++: market->setSQLitehelper(postgresHelper);
            // C++: markets.emplace_back(market);

            // C++: loggingHelper->printLog("general", 1, market->getName() + " configured successfully.");
            println!(
                "{} configured successfully.",
                market.lock().unwrap().get_spine().name
            );

            self.markets.push(market);
        }
        println!("Get entities from xml END.");
        // C++: loggingHelper->printLog("general", 1, "Configuration done.");
        println!("Configuration done.");
    }

    pub fn start(&mut self, market_startup: &str, daemon: bool) -> Vec<JoinHandle<()>> {
        // C++: sqlitePool = new ThreadPool(20);
        println!("market_startup: {}", market_startup);
        if !daemon {
            println!("daemon interface disabled");
        }
        // C++: curl = curl_easy_init();

        self.configure();

        // C++: auto ping_thread = std::async(std::launch::async, [this] {
        // C++:    this->pingPong.perform();
        // C++: });

        // C++: this->pool.perform();

        let mut threads: Vec<JoinHandle<()>> = Vec::new();

        for market in self.markets.iter().map(|a| Arc::clone(a)) {
            let market_name = market.lock().unwrap().get_spine().name.clone();
            let market_status = market.lock().unwrap().get_spine().status;
            let market_is_active = market.lock().unwrap().get_spine().status.is_active();

            println!("{} {}", market_name, market_status);

            if market_is_active
                && (market_startup.contains(&market_name) || market_startup == "all")
            {
                let thread = thread::spawn(move || {
                    let mut threads = market.lock().unwrap().perform();

                    while !threads.is_empty() {
                        let thread = threads.swap_remove(0);
                        thread.join().unwrap();
                    }
                });

                threads.push(thread);
            }
        }

        threads
    }
}
