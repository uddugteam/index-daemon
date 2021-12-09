use crate::worker::other_helpers::config_parser::ConfigParser;

const TEST_CONFIG_PATH: &str = "./test/resources/curr_daemon__good.config";
const TEST_CONFIG_PATH_2: &str = "./test/resources/curr_daemon__bad__param_code_with_space.config";
const TEST_CONFIG_PATH_3: &str = "./test/resources/curr_daemon__bad__no_param_code.config";
const TEST_CONFIG_PATH_4: &str = "./test/resources/curr_daemon__bad__no_equal_sign.config";

#[test]
fn test_config_parser() {
    let config_parser = ConfigParser::new(TEST_CONFIG_PATH).unwrap();
    assert_eq!(
        config_parser.get_param("param_code_1"),
        Some("param_value_1")
    );
    assert_eq!(
        config_parser.get_param("param_code_2"),
        Some("param_value_2")
    );
    assert_eq!(
        config_parser.get_param("param_code_3"),
        Some("param_value_3")
    );
    assert_eq!(config_parser.get_param("param_code_4"), Some(""));
    assert_eq!(config_parser.get_param("param_code_5"), Some(""));
    assert_eq!(
        config_parser.get_param("param_code_6"),
        Some("param_value 2")
    );
}

#[test]
#[should_panic]
fn test_config_parser_2() {
    let _ = ConfigParser::new(TEST_CONFIG_PATH_2);
}

#[test]
#[should_panic]
fn test_config_parser_3() {
    let _ = ConfigParser::new(TEST_CONFIG_PATH_3);
}

#[test]
#[should_panic]
fn test_config_parser_4() {
    let _ = ConfigParser::new(TEST_CONFIG_PATH_4);
}
