use std::path::Path;

use crate::worker::other_helpers::config_parser::ConfigParser;

extern crate xml;

use std::fs::File;
use std::io::BufReader;

use xml::reader::{EventReader, XmlEvent};

pub const DEFAULT_CONFIG_PATH: &str = "./resources/curr_daemon.config";
pub const DEFAULT_CONFIG_PATH_2: &str = "/etc/curr_daemon/curr_daemon.config";
pub const XML_CONFIG_FILE_PATH: &str = "./resources/config.xml";

pub struct Worker {
    configPath: Option<String>,
}

impl Worker {
    // TODO: Implement
    pub fn new() -> Worker {
        Worker { configPath: None }
    }

    fn configure(&self) {
        let path: String;

        match &self.configPath {
            Some(config_path_str) if Path::new(config_path_str).exists() => {
                path = String::from(config_path_str)
            }
            _ if Path::new(DEFAULT_CONFIG_PATH).exists() => {
                path = String::from(DEFAULT_CONFIG_PATH)
            }
            _ => path = String::from(DEFAULT_CONFIG_PATH_2),
        }

        // C++: loggingHelper->printLog("default", 1, "MainConfig path = " + path);
        println!("MainConfig path = {}", path);

        let configParser = ConfigParser::new(&path).expect("Config file open/read error.");

        let pqEnabled: bool = configParser.getParam("postgres.enabled") == Some("1");
        // C++: postgresHelper->setEnabled(pqEnabled);
        if pqEnabled {
            let pqHost = configParser
                .getParam("postgres.host")
                .expect("Param postgres.host not found");
            let pqPort = configParser
                .getParam("postgres.port")
                .expect("Param postgres.port not found")
                .parse::<i32>()
                .expect("Param postgres.port parse error");
            let pqDBName = configParser
                .getParam("postgres.dbname")
                .expect("Param postgres.dbname not found");
            let pqUser = configParser
                .getParam("postgres.username")
                .expect("Param postgres.username not found");
            let pqPass = configParser
                .getParam("postgres.password")
                .expect("Param postgres.password not found");
            let pqMaxConns = configParser
                .getParam("postgres.maxConnections")
                .and_then(|v| v.parse::<i32>().ok());
            // C++: postgresHelper->setLoggingHelper(this->loggingHelper);
            // C++: postgresHelper->init(pqHost, pqPort, pqDBName, pqUser, pqPass, pqMaxConns);
        }

        let coinsPath = configParser
            .getParam("coins.config")
            .expect("Param coins.config not found");

        // C++: TiXmlDocument doc("../resources/config.xml");

        let xml_config_file: BufReader<File>;
        match File::open(XML_CONFIG_FILE_PATH) {
            Ok(file) => {
                xml_config_file = BufReader::new(file);
            }
            Err(_) => {
                // C++: loggingHelper->printLog("default", 1, "Basic Config not Found. Checking alternative config.");
                println!("Basic Config not Found. Checking alternative config.");

                match File::open(XML_CONFIG_FILE_PATH) {
                    Ok(file) => {
                        xml_config_file = BufReader::new(file);
                    }
                    Err(_) => {
                        // C++: loggingHelper->printLog("default", 1, "Config not Found. Abort.");
                        println!("Config not Found. Abort.");
                        panic!("NoConfig");
                    }
                }
            }
        }
    }

    pub fn start(&mut self, market_startup: &str, daemon: bool, configPath: Option<String>) {
        self.configPath = configPath;
        // C++: sqlitePool = new ThreadPool(20);
        println!("market_startup: {}", market_startup);
        if !daemon {
            println!("daemon interface disabled");
        }
        // C++: curl = curl_easy_init();

        self.configure();
    }
}
