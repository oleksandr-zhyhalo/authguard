use crate::utils::errors::{Error, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs::{self, OpenOptions};
use fs2::FileExt;
use tracing::instrument;

#[derive(Debug, Serialize, Deserialize)]
pub enum CircuitState {
    Closed,
    Open {
        opened_at: DateTime<Utc>,
    },
    HalfOpen {
        attempt_count: u32,
        last_attempt: DateTime<Utc>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct CircuitBreakerState {
    state: CircuitState,
    failure_count: u32,
    last_failure: DateTime<Utc>,
    consecutive_successes: u32,
}

impl CircuitBreakerState {
    fn new() -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            last_failure: Utc::now(),
            consecutive_successes: 0,
        }
    }

    fn should_allow_request(&self, threshold: u32, cool_down_seconds: u64) -> bool {
        match &self.state {
            CircuitState::Closed => true,
            CircuitState::Open { opened_at } => {
                let cool_down_duration = Duration::seconds(cool_down_seconds as i64);
                if Utc::now() - *opened_at >= cool_down_duration {
                    true // Allow transition to half-open
                } else {
                    false
                }
            }
            CircuitState::HalfOpen { attempt_count, last_attempt } => {
                // Allow one request every 5 seconds in half-open state
                Utc::now() - *last_attempt >= Duration::seconds(5)
            }
        }
    }
}

#[instrument(skip(cache_dir))]
pub fn is_open(cache_dir: &Path, failure_threshold: u32, cool_down_seconds: u64) -> bool {
    let path = state_file_path(cache_dir);

    match read_state(&path) {
        Ok(state) => {
            let should_allow = state.should_allow_request(failure_threshold, cool_down_seconds);
            if !should_allow {
                tracing::warn!(
                    state = ?state.state,
                    failure_count = state.failure_count,
                    "Circuit breaker preventing request"
                );
            }
            !should_allow
        }
        Err(e) => {
            tracing::debug!(error = ?e, "No circuit breaker state found");
            false
        }
    }
}

#[instrument(skip(cache_dir))]
pub fn record_failure(cache_dir: &Path) -> Result<()> {
    let path = state_file_path(cache_dir);
    let mut state = read_state(&path).unwrap_or_else(|_| CircuitBreakerState::new());

    state.failure_count += 1;
    state.last_failure = Utc::now();
    state.consecutive_successes = 0;

    // Update state based on failures
    state.state = match state.state {
        CircuitState::Closed if state.failure_count >= 3 => {
            tracing::warn!("Circuit breaker transitioning to Open state");
            CircuitState::Open { opened_at: Utc::now() }
        }
        CircuitState::HalfOpen { .. } => {
            tracing::warn!("Circuit breaker returning to Open state from HalfOpen");
            CircuitState::Open { opened_at: Utc::now() }
        }
        current_state => current_state,
    };

    write_state(&path, &state)
}

#[instrument(skip(cache_dir))]
pub fn record_success(cache_dir: &Path) -> Result<()> {
    let path = state_file_path(cache_dir);
    let mut state = read_state(&path).unwrap_or_else(|_| CircuitBreakerState::new());

    state.consecutive_successes += 1;

    // Update state based on successes
    state.state = match state.state {
        CircuitState::Open { .. } => {
            tracing::info!("Circuit breaker transitioning to HalfOpen state");
            CircuitState::HalfOpen {
                attempt_count: 0,
                last_attempt: Utc::now(),
            }
        }
        CircuitState::HalfOpen { attempt_count, .. } if state.consecutive_successes >= 2 => {
            tracing::info!("Circuit breaker transitioning to Closed state");
            CircuitState::Closed
        }
        current_state => current_state,
    };

    if matches!(state.state, CircuitState::Closed) {
        state.failure_count = 0;
    }

    write_state(&path, &state)
}

fn state_file_path(cache_dir: &Path) -> PathBuf {
    cache_dir.join("cb_state.json")
}

fn read_state(path: &Path) -> Result<CircuitBreakerState> {
    if !path.exists() {
        return Ok(CircuitBreakerState::new());
    }

    let file = OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(Error::Io)?;

    file.lock_shared()
        .map_err(Error::Io)?;

    let data = fs::read_to_string(path)
        .map_err(Error::Io)?;

    let _ = file.unlock();

    serde_json::from_str(&data)
        .map_err(Error::JsonParse)
}

fn write_state(path: &Path, state: &CircuitBreakerState) -> Result<()> {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(path)
        .map_err(Error::Io)?;

    file.lock_exclusive()
        .map_err(Error::Io)?;

    let data = serde_json::to_string(state)
        .map_err(Error::JsonParse)?;

    fs::write(path, data)
        .map_err(Error::Io)?;

    let _ = file.unlock();
    Ok(())
}