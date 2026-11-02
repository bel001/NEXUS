//! Read-only, narrow WMI battery projection; never requests battery identifiers.
use adaptive_domain::telemetry::{
    PowerSource, TelemetryQuality, TelemetryReading, TelemetryReason, TelemetrySource,
};
use serde::Deserialize;
use wmi::WMIConnection;

const SOURCE: TelemetrySource = TelemetrySource::WindowsWmiBattery;
const QUERY: &str = "SELECT BatteryStatus, EstimatedChargeRemaining FROM Win32_Battery";
type Readings = (TelemetryReading<PowerSource>, TelemetryReading<u8>);

#[derive(Deserialize)]
struct BatteryRow {
    #[serde(rename = "BatteryStatus")]
    status: Option<u16>,
    #[serde(rename = "EstimatedChargeRemaining")]
    percent: Option<u16>,
}

fn failed() -> Readings {
    (
        TelemetryReading::error(SOURCE, TelemetryReason::ReadFailure),
        TelemetryReading::error(SOURCE, TelemetryReason::ReadFailure),
    )
}

fn normalize(rows: &[BatteryRow]) -> Readings {
    match rows {
        [] => (
            // Missing WMI rows do not prove absence of a physical battery or AC.
            TelemetryReading::unavailable(SOURCE, TelemetryReason::NoBattery),
            TelemetryReading::unavailable(SOURCE, TelemetryReason::NoBattery),
        ),
        [row] => {
            // Microsoft documents 1=discharging, 2=AC online, 6..=9=charging.
            // A full or partially charged battery does not prove its power source.
            let power = match row.status {
                Some(1) => TelemetryReading::power_now(PowerSource::Battery, SOURCE),
                Some(2 | 6..=9) => TelemetryReading::power_now(PowerSource::Ac, SOURCE),
                _ => TelemetryReading::unavailable(SOURCE, TelemetryReason::Unknown),
            };
            let percent = match row.percent {
                Some(value @ 0..=100) => {
                    TelemetryReading::percent_now(value as u8, SOURCE, TelemetryQuality::Estimated)
                        .expect("0..=100 is a valid battery percentage")
                }
                Some(255) => TelemetryReading::unavailable(SOURCE, TelemetryReason::Unknown),
                Some(_) => TelemetryReading::error(SOURCE, TelemetryReason::InvalidCounters),
                None => TelemetryReading::unavailable(SOURCE, TelemetryReason::Unknown),
            };
            (power, percent)
        }
        // Multiple batteries cannot be represented by a single unweighted percentage.
        _ => (
            TelemetryReading::unavailable(SOURCE, TelemetryReason::PartialScope),
            TelemetryReading::unavailable(SOURCE, TelemetryReason::PartialScope),
        ),
    }
}

fn on_wmi_thread() -> Readings {
    let connection = match WMIConnection::new() {
        Ok(connection) => connection,
        Err(_) => return failed(),
    };
    let rows: Vec<BatteryRow> = match connection.raw_query(QUERY) {
        Ok(rows) => rows,
        Err(_) => return failed(),
    };
    normalize(&rows)
}

/// One opt-in read, performed on the COM worker thread. Never returns raw WMI errors.
pub fn collect_power_telemetry() -> Readings {
    std::thread::spawn(on_wmi_thread)
        .join()
        .unwrap_or_else(|_| failed())
}

#[cfg(test)]
mod tests {
    use super::*;
    use adaptive_domain::telemetry::TelemetryStatus;

    #[test]
    fn conservative_status_mapping() {
        assert!(!QUERY.contains("SELECT *"));
        let rows = [BatteryRow {
            status: Some(1),
            percent: Some(42),
        }];
        let (power, percent) = normalize(&rows);
        assert_eq!(power.value(), Some(&PowerSource::Battery));
        assert_eq!(percent.value(), Some(&42));
        assert_eq!(percent.quality(), Some(TelemetryQuality::Estimated));
        let rows = [BatteryRow {
            status: Some(2),
            percent: Some(100),
        }];
        assert_eq!(normalize(&rows).0.value(), Some(&PowerSource::Ac));
        let rows = [BatteryRow {
            status: Some(3),
            percent: Some(80),
        }];
        assert_eq!(normalize(&rows).0.status(), TelemetryStatus::Unavailable);
    }

    #[test]
    fn missing_battery_and_invalid_percent_do_not_invent_values() {
        let (power, percent) = normalize(&[]);
        assert_eq!(power.status(), TelemetryStatus::Unavailable);
        assert_eq!(percent.status(), TelemetryStatus::Unavailable);
        let rows = [BatteryRow {
            status: None,
            percent: Some(255),
        }];
        let (power, percent) = normalize(&rows);
        assert!(power.value().is_none());
        assert_eq!(percent.status(), TelemetryStatus::Unavailable);
        let many = [
            BatteryRow {
                status: Some(1),
                percent: Some(10),
            },
            BatteryRow {
                status: Some(1),
                percent: Some(90),
            },
        ];
        assert_eq!(normalize(&many).1.status(), TelemetryStatus::Unavailable);
    }
}
