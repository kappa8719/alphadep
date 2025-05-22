use interface::RuntimeManifest;
use std::fs;

pub fn resolve_manifest() -> RuntimeManifest {
    let configuration = fs::read_to_string("alphadep-runtime.toml").unwrap();
    toml::from_str::<RuntimeManifest>(configuration.as_str()).unwrap()
}
