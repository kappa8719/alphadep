use interface::RuntimeConfiguration;
use std::fs;

pub fn resolve_configuration() -> RuntimeConfiguration {
    let configuration = fs::read_to_string("alphadep-runtime.toml").unwrap();
    toml::from_str::<RuntimeConfiguration>(configuration.as_str()).unwrap()
}
