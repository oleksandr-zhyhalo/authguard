use std::error::Error;
use tokio;
use crate::aws_iot::get_aws_credentials;

mod config;
mod aws_iot;

use crate::config::Config;

const CONFIG_PATH: &str = "remote.conf";

// Make the main function async via the tokio runtime.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 1. Load the configuration
    let config = Config::load(CONFIG_PATH)?;
    println!("Loaded config: {:?}", config);
    let _client = aws_iot::build_client(&config).await?;
    let creds =     get_aws_credentials(&config, &_client).await?;
    println!("Got credentials: {:?}", creds);

    // 3. Here you could proceed to:
    //    - Write them to .aws/credentials
    //    - Set up a refresh loop, etc.

    Ok(())
}
