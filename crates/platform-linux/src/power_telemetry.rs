//! One bounded read of Linux power_supply sysfs for TASK-002 B3.
use adaptive_domain::telemetry::{
    PowerSource, TelemetryQuality, TelemetryReading, TelemetryReason, TelemetrySource,
};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;

const SOURCE: TelemetrySource = TelemetrySource::LinuxPowerSupply;
type Readings = (TelemetryReading<PowerSource>, TelemetryReading<u8>);
const SYSFS_ROOT: &str = "/sys/class/power_supply";

fn read_attr(path: &Path, attribute: &str) -> io::Result<Option<String>> {
    let file = match File::open(path.join(attribute)) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let mut data = String::new();
    file.take(64).read_to_string(&mut data)?;
    Ok(Some(data.trim().to_owned()))
}

fn failed(error: &io::Error) -> Readings {
    if error.kind() == io::ErrorKind::PermissionDenied {
        (
            TelemetryReading::permission_denied(SOURCE),
            TelemetryReading::permission_denied(SOURCE),
        )
    } else {
        (
            TelemetryReading::error(SOURCE, TelemetryReason::ReadFailure),
            TelemetryReading::error(SOURCE, TelemetryReason::ReadFailure),
        )
    }
}

fn collect_from(root: &Path) -> Readings {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) => return failed(&error),
    };
    let mut found_battery = 0_usize;
    let mut battery_percent: Option<Result<u8, TelemetryReason>> = None;
    let mut discharging = false;
    let mut external_online = false;
    let mut external_offline = false;
    let mut invalid_power = false;

    // Reject an unexpectedly large device list rather than claiming complete coverage.
    for (index, entry) in entries.enumerate() {
        if index >= 64 {
            return (
                TelemetryReading::error(SOURCE, TelemetryReason::PartialScope),
                TelemetryReading::error(SOURCE, TelemetryReason::PartialScope),
            );
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => return failed(&error),
        };
        let path = entry.path();
        let kind = match read_attr(&path, "type") {
            Ok(Some(kind)) => kind,
            Ok(None) => continue,
            Err(error) => return failed(&error),
        };
        if kind == "Battery" {
            let present = match read_attr(&path, "present") {
                Ok(value) => value,
                Err(error) => return failed(&error),
            };
            if present.as_deref() == Some("0") {
                continue;
            }
            if !matches!(present.as_deref(), None | Some("1")) {
                invalid_power = true;
                battery_percent = Some(Err(TelemetryReason::InvalidCounters));
                continue;
            }
            found_battery += 1;
            let status = match read_attr(&path, "status") {
                Ok(value) => value,
                Err(error) => return failed(&error),
            };
            if status.as_deref() == Some("Discharging") {
                discharging = true;
            }
            let percent = match read_attr(&path, "capacity") {
                Ok(Some(value)) => value
                    .parse::<u8>()
                    .ok()
                    .filter(|value| *value <= 100)
                    .ok_or(TelemetryReason::InvalidCounters),
                Ok(None) => Err(TelemetryReason::Unknown),
                Err(error) => return failed(&error),
            };
            battery_percent = Some(percent);
        } else if kind == "Mains" || kind == "Wireless" || kind.starts_with("USB") {
            match read_attr(&path, "online") {
                Ok(Some(value)) if value == "1" => external_online = true,
                Ok(Some(value)) if value == "0" => external_offline = true,
                Ok(Some(_)) => invalid_power = true,
                Ok(None) => {}
                Err(error) => return failed(&error),
            }
        }
    }

    let power = if invalid_power || (external_online && discharging) {
        TelemetryReading::error(SOURCE, TelemetryReason::InvalidCounters)
    } else if external_online {
        TelemetryReading::power_now(PowerSource::Ac, SOURCE)
    } else if discharging && found_battery == 1 {
        TelemetryReading::power_now(PowerSource::Battery, SOURCE)
    } else if external_offline && found_battery == 0 {
        TelemetryReading::unavailable(SOURCE, TelemetryReason::NoBattery)
    } else {
        TelemetryReading::unavailable(SOURCE, TelemetryReason::Unknown)
    };
    let battery = if found_battery > 1 {
        TelemetryReading::unavailable(SOURCE, TelemetryReason::PartialScope)
    } else if found_battery == 0 && battery_percent.is_none() {
        TelemetryReading::unsupported(SOURCE, TelemetryReason::NoBattery)
    } else {
        match battery_percent {
            Some(Ok(value)) => {
                TelemetryReading::percent_now(value, SOURCE, TelemetryQuality::Estimated)
                    .expect("0..=100 is a valid percent")
            }
            Some(Err(TelemetryReason::Unknown)) | None => {
                TelemetryReading::unavailable(SOURCE, TelemetryReason::Unknown)
            }
            Some(Err(reason)) => TelemetryReading::error(SOURCE, reason),
        }
    };
    (power, battery)
}

/// No polling, external processes, network access, or writes to sysfs.
pub fn collect_power_telemetry() -> Readings {
    collect_from(Path::new(SYSFS_ROOT))
}

#[cfg(test)]
mod tests {
    use super::*;
    use adaptive_domain::telemetry::TelemetryStatus;

    #[test]
    fn ac_online_and_estimated_battery_percent() {
        let dir = tempfile::tempdir().unwrap();
        let battery = dir.path().join("BAT0");
        let ac = dir.path().join("AC0");
        fs::create_dir(&battery).unwrap();
        fs::create_dir(&ac).unwrap();
        fs::write(battery.join("type"), "Battery\n").unwrap();
        fs::write(battery.join("present"), "1\n").unwrap();
        fs::write(battery.join("capacity"), "78\n").unwrap();
        fs::write(battery.join("status"), "Charging\n").unwrap();
        fs::write(ac.join("type"), "Mains\n").unwrap();
        fs::write(ac.join("online"), "1\n").unwrap();
        let (power, percent) = collect_from(dir.path());
        assert_eq!(power.value(), Some(&PowerSource::Ac));
        assert_eq!(percent.value(), Some(&78));
        assert_eq!(percent.quality(), Some(TelemetryQuality::Estimated));
    }

    #[test]
    fn discharging_and_no_battery_are_distinguished() {
        let dir = tempfile::tempdir().unwrap();
        let battery = dir.path().join("BAT0");
        fs::create_dir(&battery).unwrap();
        fs::write(battery.join("type"), "Battery\n").unwrap();
        fs::write(battery.join("status"), "Discharging\n").unwrap();
        fs::write(battery.join("capacity"), "50\n").unwrap();
        assert_eq!(
            collect_from(dir.path()).0.value(),
            Some(&PowerSource::Battery)
        );
        fs::remove_dir_all(&battery).unwrap();
        let (power, percent) = collect_from(dir.path());
        assert_eq!(power.status(), TelemetryStatus::Unavailable);
        assert_eq!(percent.status(), TelemetryStatus::Unsupported);
    }

    #[test]
    fn invalid_percent_and_multiple_batteries_are_not_averaged() {
        let dir = tempfile::tempdir().unwrap();
        let battery = dir.path().join("BAT0");
        fs::create_dir(&battery).unwrap();
        fs::write(battery.join("type"), "Battery\n").unwrap();
        fs::write(battery.join("capacity"), "255\n").unwrap();
        assert_eq!(collect_from(dir.path()).1.status(), TelemetryStatus::Error);
        let another = dir.path().join("BAT1");
        fs::create_dir(&another).unwrap();
        fs::write(another.join("type"), "Battery\n").unwrap();
        fs::write(another.join("capacity"), "80\n").unwrap();
        assert_eq!(
            collect_from(dir.path()).1.status(),
            TelemetryStatus::Unavailable
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn live_sysfs_is_read_only_and_never_invents_a_percentage() {
        let (power, battery) = collect_power_telemetry();
        if let Some(value) = battery.value() {
            assert!(*value <= 100);
            assert_eq!(battery.status(), TelemetryStatus::Available);
        }
        if let Some(value) = power.value() {
            assert!(matches!(value, PowerSource::Ac | PowerSource::Battery));
        }
    }
}
