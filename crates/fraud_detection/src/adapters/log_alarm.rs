// Rust guideline compliant 2026-02-23

//! Demo adapter for the `Alarm` port.
//!
//! Logs fraud alerts via `log::warn!` and always returns `Ok(())`.
//! `AlarmError::DeliveryFailed` is unreachable in this demo adapter.

use domain::{Alarm, AlarmError, InferredTransaction};

/// `Alarm` adapter that emits a warning log for each fraudulent transaction.
///
/// Always returns `Ok(())`; use a custom implementation for real alerting.
#[derive(Debug)]
pub struct LogAlarm;

impl LogAlarm {
    /// Create a new log alarm adapter.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for LogAlarm {
    fn default() -> Self {
        Self::new()
    }
}

impl Alarm for LogAlarm {
    async fn trigger(&self, transaction: &InferredTransaction) -> Result<(), AlarmError> {
        log::warn!("log_alarm.fraud_alert: transaction_id={}", transaction.id());
        Ok(())
    }
}
