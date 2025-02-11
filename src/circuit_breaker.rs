use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize)]
struct CircuitBreakerState {
    failure_count: u32,
    last_failure: DateTime<Utc>,
}

pub fn is_open(cache_dir: &Path, failure_threshold: u32, cool_down_seconds: u64) -> bool {
    let path = state_file_path(cache_dir);
    if let Ok(state) = read_state(&path) {
        if state.failure_count >= failure_threshold {
            let elapsed = Utc::now() - state.last_failure;
            if elapsed < Duration::seconds(cool_down_seconds as i64) {
                tracing::warn!(
                    "Circuit breaker is open (last failure {} seconds ago)",
                    elapsed.num_seconds()
                );
                return true;
            }
        }
    }
    false
}

pub fn record_failure(cache_dir: &Path) {
    let path = state_file_path(cache_dir);
    let mut state = read_state(&path).unwrap_or_else(|_| CircuitBreakerState {
        failure_count: 0,
        last_failure: Utc::now(),
    });

    state.failure_count += 1;
    state.last_failure = Utc::now();
    let _ = write_state(&path, &state);
}

pub fn record_success(cache_dir: &Path) {
    let path = state_file_path(cache_dir);
    let _ = fs::remove_file(path);
}

fn state_file_path(cache_dir: &Path) -> PathBuf {
    cache_dir.join("cb_state.json")
}

fn read_state(path: &Path) -> Result<CircuitBreakerState> {
    if !path.exists() {
        anyhow::bail!("No circuit breaker state file");
    }

    let file = OpenOptions::new()
        .read(true)
        .open(path)
        .context("Failed to open circuit breaker state file")?;

    file.lock_shared()
        .context("Failed to acquire shared lock on circuit breaker file")?;

    let data = fs::read_to_string(path).context("Failed to read circuit breaker state file")?;

    file.unlock().ok();
    serde_json::from_str(&data).context("Failed to parse circuit breaker state")
}

fn write_state(path: &Path, state: &CircuitBreakerState) -> Result<()> {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(path)
        .context("Failed to open circuit breaker state file for writing")?;

    file.lock_exclusive()
        .context("Failed to acquire exclusive lock on circuit breaker state file")?;

    let data = serde_json::to_string(state).context("Failed to serialize circuit breaker state")?;

    fs::write(path, data).context("Failed to write circuit breaker state")?;

    file.unlock().ok();
    Ok(())
}
