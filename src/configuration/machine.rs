use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SSHIdentityConfiguration {
    Key { path: String },
    Password { value: String },
}

#[derive(Deserialize, Debug, Clone)]
pub struct SSHMachineConfiguration {
    pub host: String,
    pub user: String,
    pub identity: SSHIdentityConfiguration,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum MachineConfiguration {
    #[serde(rename = "remote/ssh")]
    RemoteSSH(SSHMachineConfiguration),
}
