use config::Config;
use config::ConfigError;
use reqwest::blocking::Client;
use serde::Deserialize;
use serde::Serialize;
use simple_error::bail;
use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

extern crate reqwest;

const CF_API_URL_BASE: &str = "https://api.cloudflare.com/client/v4";

type BoxResult<T> = Result<T,Box<dyn Error>>;

#[derive(Deserialize)]
struct AppConfig {
    interval: u64,
    zone_id: String,
    record_id: String,
    token: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ARecord {
    comment: Option<String>,
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

struct UpdateResult {
    ip: String,
    changed: bool,
}

fn build_config() -> Result<Config, ConfigError> {
    Config::builder()
        .add_source(config::Environment::with_prefix("CF"))
        .set_default("INTERVAL", 120)?
        .build()
}

fn get_public_ip(client: &Client) -> BoxResult<String> {
    let body = client.get("https://cloudflare.com/cdn-cgi/trace")
        .send().map_err(|err| format!("error getting public ip: {err}"))?
        .text().map_err(|err| format!("error getting public ip: {err}"))?;

    let lines = body.split("\n");
    for line in lines {
        let fields: Vec<&str> = line.split("=").collect();
        if fields.len() == 2 && fields[0] == "ip" {
            return Ok(fields[1].to_string());
        }
    }

    bail!("ip address not found in response");
}

fn get_dns_record(client: &Client, config: &AppConfig) -> BoxResult<ARecord> {
    let resp = client.get(format!("{}/zones/{}/dns_records/{}",
        CF_API_URL_BASE, config.zone_id, config.record_id))
        .bearer_auth(&config.token)
        .send()
        .map_err(|err| format!("error getting dns record: {err}"))?;

    let status_code = resp.status().as_u16();
    if status_code != 200 {
        bail!(format!("unexpected status code while getting dns record: {}", status_code));
    }

    let result: DnsResponse = resp.json().map_err(|err| format!("error getting dns record: {err}"))?;
    if !result.success {
        bail!("dns get result was unsuccessful");
    }

    Ok(result.result)
}

fn update_dns_record(record: &ARecord, client: &Client, config: &AppConfig) -> BoxResult<()> {
    let res = client.patch(format!("{}/zones/{}/dns_records/{}",
            CF_API_URL_BASE, config.zone_id, config.record_id))
        .bearer_auth(&config.token)
        .json(record)
        .send()
        .map_err(|err| format!("error updating dns record: {err}"))?;

    let status_code = res.status().as_u16();
    if status_code != 200 {
        bail!(format!("unexpected status code while updating dns record: {}", status_code));
    }

    Ok(())
}

fn update_ip(new_ip: &String, client: &Client, config: &AppConfig) -> BoxResult<bool> {
    let mut record = get_dns_record(client, config)?;
    if record.content == *new_ip {
        return Ok(false);
    }

    record.content = new_ip.to_string();
    update_dns_record(&record, client, config)?;

    Ok(true)
}

fn check_and_update(cached_ip: &String, client: &Client, config: &AppConfig) -> BoxResult<UpdateResult> {
    let ip = get_public_ip(client)?;
    if *cached_ip == ip {
        return Ok(UpdateResult { ip: ip.to_string(), changed: false })
    }

    let changed = update_ip(&ip, client, config)?;
    Ok(UpdateResult { ip: ip.to_string(), changed })
}

fn main() {
    println!("Starting...");

    let client = Client::new();
    let config: AppConfig = build_config()
        .unwrap()
        .try_deserialize()
        .unwrap();

    println!("Using config:");
    println!("CF_INTERVAL: {}", config.interval);
    println!("CF_ZONE_ID: {}", config.zone_id);
    println!("CF_RECORD_ID: {}", config.record_id);
    let token_redacted: String = config.token.chars().map(|_| "*").collect();
    println!("CF_TOKEN: {token_redacted}");

    let mut cached_ip = "".to_string();

    println!("Loop started.");
    loop {
        match check_and_update(&cached_ip, &client, &config) {
            Ok(res) => {
                if res.changed {
                    cached_ip = res.ip;
                    println!("Updated ip address to {cached_ip}");
                } else {
                    println!("No update necessary");
                }
            },
            Err(err) => println!("{err}"),
        }
        sleep(Duration::from_secs(config.interval))
    }
}
