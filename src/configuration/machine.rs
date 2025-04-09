use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SSHIdentityConfiguration {
    Key { path: String },
    Password { value: String },
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct SSHRuntimeConfiguration {
    #[serde(rename = "always-update")]
    pub always_update: bool,
    pub temporary: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SSHMachineConfiguration {
    pub host: String,
    pub user: String,
    pub identity: SSHIdentityConfiguration,
    #[serde(default)]
    pub runtime: SSHRuntimeConfiguration,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum MachineConfiguration {
    #[serde(rename = "remote/ssh")]
    RemoteSSH(SSHMachineConfiguration),
}
