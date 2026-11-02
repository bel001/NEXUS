//! Read-only WMI raw CPU counters; no FFI, no unsafe code, no external process.
use adaptive_domain::telemetry::{
    CpuScope, CpuUsageReading, TelemetryReason, TelemetrySource, windows_wmi_cpu_percent,
};
use serde::Deserialize;
use std::thread;
use std::time::{Duration, Instant};
use wmi::WMIConnection;

const SOURCE: TelemetrySource = TelemetrySource::WindowsWmiCpu;
const WINDOW_MS: u64 = 250;
const MAX_WINDOW_MS: u64 = 10_000;
const COUNTER_QUERY: &str = "SELECT Name, PercentProcessorTime, Timestamp_Sys100NS FROM Win32_PerfRawData_PerfOS_Processor WHERE Name = '_Total'";
const SCOPE_QUERY: &str = "SELECT NumberOfLogicalProcessors FROM Win32_ComputerSystem";

#[derive(Deserialize)]
struct RawCpu {
    #[serde(rename = "Name")]
    name: Option<String>,
    #[serde(rename = "PercentProcessorTime")]
    idle_counter: Option<u64>,
    #[serde(rename = "Timestamp_Sys100NS")]
    timestamp: Option<u64>,
}
#[derive(Deserialize)]
struct ScopeRow {
    #[serde(rename = "NumberOfLogicalProcessors")]
    logical_processors: Option<u32>,
}
fn parse_raw(rows: &[RawCpu]) -> Result<(u64, u64), TelemetryReason> {
    match rows {
        [row] if row.name.as_deref() == Some("_Total") => match (row.idle_counter, row.timestamp) {
            (Some(idle), Some(time)) => Ok((idle, time)),
            _ => Err(TelemetryReason::InvalidCounters),
        },
        _ => Err(TelemetryReason::InvalidCounters),
    }
}
fn measure(previous: (u64, u64), current: (u64, u64), window: u64) -> CpuUsageReading {
    if !(100..=MAX_WINDOW_MS).contains(&window) {
        return CpuUsageReading::error(
            SOURCE,
            CpuScope::SystemTotal,
            TelemetryReason::InvalidWindow,
        );
    }
    match windows_wmi_cpu_percent(previous.0, previous.1, current.0, current.1) {
        Ok(percent) => {
            CpuUsageReading::available_now(percent, window, CpuScope::SystemTotal, SOURCE)
                .unwrap_or_else(|_| {
                    CpuUsageReading::error(
                        SOURCE,
                        CpuScope::SystemTotal,
                        TelemetryReason::InvalidCounters,
                    )
                })
        }
        Err(_) => CpuUsageReading::error(
            SOURCE,
            CpuScope::SystemTotal,
            TelemetryReason::InvalidCounters,
        ),
    }
}
fn on_wmi_thread() -> CpuUsageReading {
    let connection = match WMIConnection::new() {
        Ok(connection) => connection,
        Err(_) => {
            return CpuUsageReading::error(SOURCE, CpuScope::Unknown, TelemetryReason::ReadFailure);
        }
    };
    // Conservatively reject machines with more than 64 logical processors:
    // the classic PerfOS Processor _Total counter is not certified here across processor groups.
    let scope_rows: Vec<ScopeRow> = match connection.raw_query(SCOPE_QUERY) {
        Ok(rows) => rows,
        Err(_) => {
            return CpuUsageReading::error(SOURCE, CpuScope::Unknown, TelemetryReason::ReadFailure);
        }
    };
    let count = match scope_rows.as_slice() {
        [row] => row.logical_processors,
        _ => None,
    };
    match count {
        Some(1..=64) => {}
        Some(_) => {
            return CpuUsageReading::unavailable(
                SOURCE,
                CpuScope::ProcessorGroup,
                TelemetryReason::PartialScope,
            );
        }
        None => {
            return CpuUsageReading::unavailable(
                SOURCE,
                CpuScope::Unknown,
                TelemetryReason::PartialScope,
            );
        }
    }
    let first: Vec<RawCpu> = match connection.raw_query(COUNTER_QUERY) {
        Ok(rows) => rows,
        Err(_) => {
            return CpuUsageReading::error(SOURCE, CpuScope::Unknown, TelemetryReason::ReadFailure);
        }
    };
    let first = match parse_raw(&first) {
        Ok(value) => value,
        Err(TelemetryReason::PermissionDenied) => {
            return CpuUsageReading::permission_denied(SOURCE, CpuScope::Unknown);
        }
        Err(reason) => return CpuUsageReading::error(SOURCE, CpuScope::Unknown, reason),
    };
    let start = Instant::now();
    thread::sleep(Duration::from_millis(WINDOW_MS));
    let second: Vec<RawCpu> = match connection.raw_query(COUNTER_QUERY) {
        Ok(rows) => rows,
        Err(_) => {
            return CpuUsageReading::error(SOURCE, CpuScope::Unknown, TelemetryReason::ReadFailure);
        }
    };
    let second = match parse_raw(&second) {
        Ok(value) => value,
        Err(TelemetryReason::PermissionDenied) => {
            return CpuUsageReading::permission_denied(SOURCE, CpuScope::Unknown);
        }
        Err(reason) => return CpuUsageReading::error(SOURCE, CpuScope::Unknown, reason),
    };
    let window = match u64::try_from(start.elapsed().as_millis()) {
        Ok(value) => value,
        Err(_) => {
            return CpuUsageReading::error(
                SOURCE,
                CpuScope::SystemTotal,
                TelemetryReason::InvalidWindow,
            );
        }
    };
    measure(first, second, window)
}
/// A single explicit request, with WMI owned by its worker thread.
pub fn collect_cpu_telemetry() -> CpuUsageReading {
    thread::spawn(on_wmi_thread).join().unwrap_or_else(|_| {
        CpuUsageReading::error(SOURCE, CpuScope::Unknown, TelemetryReason::ReadFailure)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use adaptive_domain::telemetry::TelemetryStatus;

    #[test]
    fn parses_only_total_with_complete_raw_fields() {
        let row = RawCpu {
            name: Some("_Total".into()),
            idle_counter: Some(100),
            timestamp: Some(200),
        };
        assert_eq!(parse_raw(&[row]), Ok((100, 200)));
        let wrong = RawCpu {
            name: Some("0".into()),
            idle_counter: Some(100),
            timestamp: Some(200),
        };
        assert_eq!(parse_raw(&[wrong]), Err(TelemetryReason::InvalidCounters));
        let absent = RawCpu {
            name: Some("_Total".into()),
            idle_counter: None,
            timestamp: Some(200),
        };
        assert_eq!(parse_raw(&[absent]), Err(TelemetryReason::InvalidCounters));
    }
    #[test]
    fn calculates_counter_window_without_clamping() {
        let cpu = measure((10, 100), (90, 200), 250);
        assert_eq!(cpu.usage_percent().status(), TelemetryStatus::Available);
        assert_eq!(cpu.usage_percent().value(), Some(&20.0));
        let invalid = measure((10, 100), (120, 200), 250);
        assert_eq!(invalid.usage_percent().status(), TelemetryStatus::Error);
        let invalid_window = measure((10, 100), (90, 200), 0);
        assert_eq!(
            invalid_window.usage_percent().status(),
            TelemetryStatus::Error
        );
    }
}
