use std::{fs::File, io::Read};

use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub listener: Option<String>,
    pub remote: Remote,
    pub tls: Option<Tls>,
    pub auth: Option<Auth>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Remote {
    pub host: String,
    pub port: u16,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Tls {
    pub enabled: bool,
    pub ca_cert: Option<std::path::PathBuf>,
}
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Auth {
    #[serde(flatten)]
    pub auth: AuthMethod,
}
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub enum AuthMethod {
    BasicAuth { username: String, password: String },
    #[serde(rename_all = "camelCase")]
    Jwt { token_file: std::path::PathBuf },
}

pub fn load_config() -> Config {
    let path = std::env::var("CONFIG_FILE").unwrap_or(String::from("./config.yaml"));
    let mut f = File::open(path).expect("file not found");
    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .expect("something went wrong reading the file");
    serde_yaml::from_str(&contents).unwrap()
}
