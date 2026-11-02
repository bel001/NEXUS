//! Local memory telemetry from the read-only Linux proc filesystem.
use adaptive_domain::telemetry::{
    TelemetryQuality, TelemetryReading, TelemetryReason, TelemetrySource,
};
use std::fs;

const SOURCE: TelemetrySource = TelemetrySource::LinuxProcMeminfo;
type Readings = (TelemetryReading<u64>, TelemetryReading<u64>);

fn absent() -> TelemetryReading<u64> {
    TelemetryReading::unavailable(SOURCE, TelemetryReason::ReadFailure)
}
fn invalid() -> TelemetryReading<u64> {
    TelemetryReading::error(SOURCE, TelemetryReason::InvalidMemory)
}
fn field(input: &str, name: &str) -> Result<Option<u64>, ()> {
    let mut value = None;
    for line in input.lines() {
        let Some((key, rest)) = line.split_once(':') else {
            continue;
        };
        if key.trim() != name {
            continue;
        }
        if value.is_some() {
            return Err(());
        }
        let mut parts = rest.split_whitespace();
        let amount = parts.next().ok_or(())?.parse::<u64>().map_err(|_| ())?;
        if parts.next() != Some("kB") || parts.next().is_some() {
            return Err(());
        }
        value = Some(amount.checked_mul(1024).ok_or(())?);
    }
    Ok(value)
}
fn reading(parsed: Result<Option<u64>, ()>, quality: TelemetryQuality) -> TelemetryReading<u64> {
    match parsed {
        Ok(Some(value)) => TelemetryReading::bytes_now(value, SOURCE, quality),
        Ok(None) => absent(),
        Err(()) => invalid(),
    }
}
fn parse(input: &str) -> Readings {
    let total = field(input, "MemTotal");
    let available = field(input, "MemAvailable");
    let contradictory = match (&total, &available) {
        (Ok(Some(0)), _) => true,
        (Ok(Some(t)), Ok(Some(a))) => a > t,
        _ => false,
    };
    if contradictory {
        return (invalid(), invalid());
    }
    (
        reading(total, TelemetryQuality::Measured),
        reading(available, TelemetryQuality::Estimated),
    )
}
/// One explicit read; no process execution, polling, or network access.
pub fn collect_memory_telemetry() -> Readings {
    match fs::read_to_string("/proc/meminfo") {
        Ok(input) => parse(&input),
        Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => (
            TelemetryReading::permission_denied(SOURCE),
            TelemetryReading::permission_denied(SOURCE),
        ),
        Err(_) => (
            TelemetryReading::error(SOURCE, TelemetryReason::ReadFailure),
            TelemetryReading::error(SOURCE, TelemetryReason::ReadFailure),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adaptive_domain::telemetry::TelemetryStatus;

    #[test]
    fn parses_kib_and_ignores_other_lines() {
        let (total, available) = parse("MemFree: 1 kB\nMemTotal: 8192 kB\nMemAvailable: 2048 kB\n");
        assert_eq!(total.value(), Some(&8_388_608));
        assert_eq!(available.value(), Some(&2_097_152));
        assert_eq!(available.status(), TelemetryStatus::Available);
        assert!(available.measured_at().is_some());
    }
    #[test]
    fn missing_and_malformed_fields_are_distinct() {
        let (total, available) = parse("MemTotal: 100 kB\n");
        assert!(total.value().is_some());
        assert_eq!(available.status(), TelemetryStatus::Unavailable);
        let (total, available) = parse("MemTotal: 100 MB\nMemAvailable: 10 kB\n");
        assert_eq!(total.status(), TelemetryStatus::Error);
        assert!(available.value().is_some());
    }
    #[test]
    fn rejects_contradictions_duplicates_and_overflow() {
        for input in [
            "MemTotal: 5 kB\nMemAvailable: 10 kB\n",
            "MemTotal: 0 kB\nMemAvailable: 0 kB\n",
        ] {
            let (total, available) = parse(input);
            assert_eq!(total.status(), TelemetryStatus::Error);
            assert_eq!(available.status(), TelemetryStatus::Error);
        }
        let (total, _) = parse("MemTotal: 1 kB\nMemTotal: 2 kB\n");
        assert_eq!(total.status(), TelemetryStatus::Error);
        let (total, _) = parse("MemTotal: 18446744073709551615 kB\n");
        assert_eq!(total.status(), TelemetryStatus::Error);
    }
}
