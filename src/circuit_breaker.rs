// src/circuit_breaker.rs
use anyhow::{Context, Result};
use chrono::{DateTime, Utc, Duration};
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::fs::{self, OpenOptions};
use fs2::FileExt;

const CB_STATE_FILE: &str = "/var/cache/authguard/cb_state.json";
const FAILURE_THRESHOLD: u32 = 3;
const COOL_DOWN_SECS: i64 = 60;

#[derive(Serialize, Deserialize)]
struct CircuitBreakerState {
    failure_count: u32,
    last_failure: DateTime<Utc>,
}

fn state_file_path() -> PathBuf {
    PathBuf::from(CB_STATE_FILE)
}

pub fn is_open() -> bool {
    if let Ok(state) = read_state() {
        if state.failure_count >= FAILURE_THRESHOLD {
            let elapsed = Utc::now() - state.last_failure;
            if elapsed < Duration::seconds(COOL_DOWN_SECS) {
                tracing::warn!("Circuit breaker is open (last failure {} seconds ago)", elapsed.num_seconds());
                return true;
            }
        }
    }
    false
}

pub fn record_failure() {
    let mut state = read_state().unwrap_or(CircuitBreakerState {
        failure_count: 0,
        last_failure: Utc::now(),
    });
    state.failure_count += 1;
    state.last_failure = Utc::now();
    let _ = write_state(&state);
}

pub fn record_success() {
    let _ = fs::remove_file(state_file_path());
}

fn read_state() -> Result<CircuitBreakerState> {
    let path = state_file_path();
    if !path.exists() {
        anyhow::bail!("No circuit breaker state file");
    }
    let file = OpenOptions::new().read(true).open(&path)
        .with_context(|| "Failed to open circuit breaker state file")?;
    file.lock_shared().with_context(|| "Failed to acquire shared lock on circuit breaker file")?;
    let data = std::fs::read_to_string(&path)
        .with_context(|| "Failed to read circuit breaker state file")?;
    file.unlock().ok();
    let state: CircuitBreakerState = serde_json::from_str(&data)
        .with_context(|| "Failed to parse circuit breaker state")?;
    Ok(state)
}

fn write_state(state: &CircuitBreakerState) -> Result<()> {
    let path = state_file_path();
    let file = OpenOptions::new().write(true).create(true).open(&path)
        .with_context(|| "Failed to open circuit breaker state file for writing")?;
    file.lock_exclusive().with_context(|| "Failed to acquire exclusive lock on circuit breaker state file")?;
    let data = serde_json::to_string(state)
        .with_context(|| "Failed to serialize circuit breaker state")?;
    std::fs::write(&path, data)
        .with_context(|| "Failed to write circuit breaker state")?;
    file.unlock().ok();
    Ok(())
}
