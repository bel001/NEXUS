//! Explicit and limited read-only WMI memory query for TASK-002 B1.
use adaptive_domain::telemetry::{
    TelemetryQuality, TelemetryReading, TelemetryReason, TelemetrySource,
};
use serde::Deserialize;
use wmi::WMIConnection;

const SOURCE: TelemetrySource = TelemetrySource::WindowsWmiMemory;
const QUERY: &str = "SELECT TotalVisibleMemorySize, FreePhysicalMemory FROM Win32_OperatingSystem";
type Readings = (TelemetryReading<u64>, TelemetryReading<u64>);

#[derive(Deserialize)]
struct MemoryRow {
    #[serde(rename = "TotalVisibleMemorySize")]
    total_visible_memory_size: Option<u64>,
    #[serde(rename = "FreePhysicalMemory")]
    free_physical_memory: Option<u64>,
}
fn invalid() -> TelemetryReading<u64> {
    TelemetryReading::error(SOURCE, TelemetryReason::InvalidMemory)
}
fn failed() -> Readings {
    (
        TelemetryReading::error(SOURCE, TelemetryReason::ReadFailure),
        TelemetryReading::error(SOURCE, TelemetryReason::ReadFailure),
    )
}
fn normalize(total_kib: Option<u64>, available_kib: Option<u64>) -> Readings {
    let total = total_kib.and_then(|v| v.checked_mul(1024));
    let available = available_kib.and_then(|v| v.checked_mul(1024));
    if total == Some(0) || matches!((total, available), (Some(t), Some(a)) if a > t) {
        return (invalid(), invalid());
    }
    let read = |raw: Option<u64>, bytes: Option<u64>| match (raw, bytes) {
        (Some(_), Some(value)) => {
            TelemetryReading::bytes_now(value, SOURCE, TelemetryQuality::Measured)
        }
        (None, _) => TelemetryReading::unavailable(SOURCE, TelemetryReason::ReadFailure),
        (Some(_), None) => invalid(),
    };
    (read(total_kib, total), read(available_kib, available))
}
fn on_wmi_thread() -> Readings {
    let connection = match WMIConnection::new() {
        Ok(connection) => connection,
        Err(_) => return failed(),
    };
    let rows: Vec<MemoryRow> = match connection.raw_query(QUERY) {
        Ok(rows) => rows,
        Err(_) => return failed(),
    };
    match rows.as_slice() {
        [row] => normalize(row.total_visible_memory_size, row.free_physical_memory),
        [] => (
            TelemetryReading::unavailable(SOURCE, TelemetryReason::ReadFailure),
            TelemetryReading::unavailable(SOURCE, TelemetryReason::ReadFailure),
        ),
        _ => (invalid(), invalid()),
    }
}
/// COM is initialized on its own worker thread, following Hardware Discovery.
/// Never exposes raw WMI error messages or identifiers.
pub fn collect_memory_telemetry() -> Readings {
    std::thread::spawn(on_wmi_thread)
        .join()
        .unwrap_or_else(|_| failed())
}

#[cfg(test)]
mod tests {
    use super::*;
    use adaptive_domain::telemetry::TelemetryStatus;
    #[test]
    fn converts_kib_and_keeps_a_timestamp() {
        assert!(QUERY.starts_with("SELECT "));
        assert!(!QUERY.contains("SELECT *"));
        let (total, available) = normalize(Some(1024), Some(256));
        assert_eq!(total.value(), Some(&1_048_576));
        assert_eq!(available.value(), Some(&262_144));
        assert!(available.measured_at().is_some());
    }
    #[test]
    fn handles_missing_invalid_and_overflow() {
        let (total, available) = normalize(None, Some(10));
        assert_eq!(total.status(), TelemetryStatus::Unavailable);
        assert_eq!(available.status(), TelemetryStatus::Available);
        let (total, available) = normalize(Some(10), Some(20));
        assert_eq!(total.status(), TelemetryStatus::Error);
        assert_eq!(available.status(), TelemetryStatus::Error);
        let (total, _) = normalize(Some(u64::MAX), None);
        assert_eq!(total.status(), TelemetryStatus::Error);
    }
}
