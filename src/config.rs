use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Deserialize, Default, Debug)]
pub struct Config {
    pub aws_iot_endpoint: String,
    pub role_alias: String,
    pub cert_path: String,
    pub key_path: String,
    pub ca_path: String,
    pub refresh_interval_secs: u64,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: Self = content
            .lines()
            .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
            .filter_map(|line| {
                let mut parts = line.splitn(2, '=');
                Some((
                    parts.next()?.trim().to_string(),
                    parts.next()?.trim().to_string(),
                ))
            })
            .try_fold(
                Self::default(),
                |mut config, (key, value)| -> Result<Self, Box<dyn std::error::Error>> {
                    match key.as_str() {
                        "aws_iot_endpoint" => config.aws_iot_endpoint = value,
                        "role_alias" => config.role_alias = value,
                        "cert_path" => config.cert_path = value,
                        "key_path" => config.key_path = value,
                        "ca_path" => config.ca_path = value,
                        "refresh_interval_secs" => config.refresh_interval_secs = value.parse()?,
                        _ => {}
                    }
                    Ok(config)
                },
            )?;

        Ok(config)
    }
}