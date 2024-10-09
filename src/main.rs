use config::Config;
use config::ConfigError;
use reqwest::blocking::Client;
use serde::Deserialize;
use serde::Serialize;
use simple_error::bail;
use std::error::Error;

extern crate reqwest;

const CF_API_URL_BASE: &str = "https://api.cloudflare.com/client/v4";

#[derive(Deserialize)]
struct AppConfig {
    interval: u64,
    zone_id: String,
    record_id: String,
    token: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct ARecord {
    comment: String,
    name: String,
    proxied: bool,
    settings: serde_json::Value,
    tags: Vec<String>,
    ttl: u32,
    content: String,
    #[serde(alias="type")]
    record_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct DnsResponse {
    errors: serde_json::Value,
    messages: serde_json::Value,
    success: bool,
    result: ARecord,
}

fn build_config() -> Result<Config, ConfigError> {
    Config::builder()
        .add_source(config::Environment::with_prefix("CF"))
        .set_default("INTERVAL", 120)?
        .build()
}

fn get_public_ip(client: &Client) -> Result<String, Box<dyn Error>> {
    let body = client.get("https://cloudflare.com/cdn-cgi/trace")
        .send()?
        .text()?;

    let lines = body.split("\n");
    for line in lines {
        let fields: Vec<&str> = line.split("=").collect();
        if fields.len() == 2 && fields[0] == "ip" {
            return Ok(fields[1].to_string());
        }
    }

    bail!("ip address not found in response");
}

fn get_dns_record(client: &Client, config: &AppConfig) -> Result<ARecord, Box<dyn Error>> {
    let resp = client.get(format!("{}/zones/{}/dns_records/{}",
        CF_API_URL_BASE, config.zone_id, config.record_id))
        .bearer_auth(&config.token)
        .send()?;

    let status_code = resp.status().as_u16();
    if status_code != 200 {
        bail!(format!("got unexpected response code while getting dns record: {}", status_code));
    }

    let result: DnsResponse = resp.json()?;
    if !result.success {
        bail!("dns get result was unsuccessful");
    }

    Ok(result.result)
}

fn update_ip(new_ip: &String, client: &Client, config: &AppConfig) -> Option<Box<dyn Error>> {
    let mut record = get_dns_record(client, config);

    None
}

fn check_and_update(cached_ip: &String, client: &Client, config: &AppConfig) -> Result<String,Box<dyn Error>> {
    let ip = get_public_ip(client)?;
    if *cached_ip == ip {
        return Ok(ip)
    }

    if let Some(err) = update_ip(&ip, client, config) {
        return Err(err);
    }

    Ok(ip)
}

fn main() {
    let client = Client::new();
    let config: AppConfig = build_config()
        .unwrap()
        .try_deserialize()
        .unwrap();
    let mut cached_ip = "".to_string();

    loop {
        match check_and_update(&cached_ip, &client, &config) {
            Ok(ip) => cached_ip = ip,
            Err(err) => println!("{err}"),
        }
        std::thread::sleep(std::time::Duration::from_secs(config.interval))
    }
}
