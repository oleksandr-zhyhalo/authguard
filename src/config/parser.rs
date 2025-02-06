use crate::config::PathBuf;
use crate::config::Config;
use crate::error::{ConfigError, Error};

pub fn parse(content: &str) -> Result<Config, Error> {
    let mut config = Config {
        aws_iot_endpoint: String::new(),
        role_alias: String::new(),
        cert_path: PathBuf::new(),
        key_path: PathBuf::new(),
        ca_path: PathBuf::new(),
    };

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.splitn(2, '=');
        let key = parts.next().ok_or_else(|| ConfigError::InvalidFormat("Invalid line format".into()))?.trim();
        let value = parts.next().ok_or_else(|| ConfigError::InvalidFormat("Missing value".into()))?.trim();

        match key {
            "aws_iot_endpoint" => config.aws_iot_endpoint = value.to_string(),
            "role_alias" => config.role_alias = value.to_string(),
            "cert_path" => config.cert_path = PathBuf::from(value),
            "key_path" => config.key_path = PathBuf::from(value),
            "ca_path" => config.ca_path = PathBuf::from(value),
            _ => return Err(ConfigError::InvalidFormat(format!("Unknown key: {}", key)).into()),
        }
    }

    validate_required_fields(&config)?;
    Ok(config)
}

fn validate_required_fields(config: &Config) -> Result<(), ConfigError> {
    if config.aws_iot_endpoint.is_empty() {
        return Err(ConfigError::MissingField("aws_iot_endpoint".into()));
    }
    if config.role_alias.is_empty() {
        return Err(ConfigError::MissingField("role_alias".into()));
    }
    Ok(())
}