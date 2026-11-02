//! Read-only global CPU usage from two bounded /proc/stat samples.
use adaptive_domain::telemetry::{
    CpuScope, CpuUsageReading, LinuxCpuTimes, TelemetryReason, TelemetrySource, linux_cpu_percent,
};
use std::fs::File;
use std::io::{self, Read};
use std::thread;
use std::time::{Duration, Instant};

const SOURCE: TelemetrySource = TelemetrySource::LinuxProcStat;
const WINDOW_MS: u64 = 250;
const MAX_WINDOW_MS: u64 = 10_000;

/// Ignore guest and guest_nice because they are already included in user/nice.
fn parse(input: &str) -> Result<LinuxCpuTimes, TelemetryReason> {
    let line = input
        .lines()
        .next()
        .ok_or(TelemetryReason::InvalidCounters)?;
    let mut fields = line.split_whitespace();
    if fields.next() != Some("cpu") {
        return Err(TelemetryReason::InvalidCounters);
    }
    let mut next = || {
        fields
            .next()
            .ok_or(TelemetryReason::InvalidCounters)?
            .parse::<u64>()
            .map_err(|_| TelemetryReason::InvalidCounters)
    };
    Ok(LinuxCpuTimes {
        user: next()?,
        nice: next()?,
        system: next()?,
        idle: next()?,
        iowait: next()?,
        irq: next()?,
        softirq: next()?,
        steal: next()?,
    })
}

fn read_sample() -> Result<LinuxCpuTimes, TelemetryReason> {
    let file = File::open("/proc/stat").map_err(|err| {
        if err.kind() == io::ErrorKind::PermissionDenied {
            TelemetryReason::PermissionDenied
        } else {
            TelemetryReason::ReadFailure
        }
    })?;
    // Only the first aggregate line is needed. Limit each read to 512 bytes.
    let mut bytes = Vec::new();
    file.take(512)
        .read_to_end(&mut bytes)
        .map_err(|_| TelemetryReason::ReadFailure)?;
    let text = std::str::from_utf8(&bytes).map_err(|_| TelemetryReason::InvalidCounters)?;
    parse(text)
}

/// A single opt-in measurement, never background polling.
pub fn collect_cpu_telemetry() -> CpuUsageReading {
    let first = match read_sample() {
        Ok(value) => value,
        Err(TelemetryReason::PermissionDenied) => {
            return CpuUsageReading::permission_denied(SOURCE, CpuScope::Unknown);
        }
        Err(reason) => return CpuUsageReading::error(SOURCE, CpuScope::Unknown, reason),
    };
    let start = Instant::now();
    thread::sleep(Duration::from_millis(WINDOW_MS));
    let second = match read_sample() {
        Ok(value) => value,
        Err(TelemetryReason::PermissionDenied) => {
            return CpuUsageReading::permission_denied(SOURCE, CpuScope::Unknown);
        }
        Err(reason) => return CpuUsageReading::error(SOURCE, CpuScope::Unknown, reason),
    };
    let window = match u64::try_from(start.elapsed().as_millis()) {
        Ok(value) if (100..=MAX_WINDOW_MS).contains(&value) => value,
        _ => {
            return CpuUsageReading::error(
                SOURCE,
                CpuScope::SystemTotal,
                TelemetryReason::InvalidWindow,
            );
        }
    };
    match linux_cpu_percent(first, second) {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "linux")]
    use adaptive_domain::telemetry::TelemetryStatus;

    #[test]
    fn parses_global_counter_and_ignores_guest() {
        let parsed = parse("cpu 10 20 30 40 50 60 70 80 90 100\ncpu0 1 2 3 4 5 6 7 8\n").unwrap();
        assert_eq!(parsed.user, 10);
        assert_eq!(parsed.steal, 80);
        assert_eq!(parsed.iowait, 50);
    }
    #[test]
    fn rejects_truncated_invalid_and_per_core_lines() {
        for input in [
            "",
            "cpu0 1 2 3 4 5 6 7 8\n",
            "cpu 1 2 3\n",
            "cpu 1 2 xx 4 5 6 7 8\n",
        ] {
            assert_eq!(parse(input), Err(TelemetryReason::InvalidCounters));
        }
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn live_linux_proc_stat_read_only() {
        let counters = read_sample().expect("/proc/stat aggregate readable");
        assert!(counters.user > 0 || counters.system > 0 || counters.idle > 0);
        let cpu = collect_cpu_telemetry();
        if cpu.usage_percent().status() == TelemetryStatus::Available {
            assert_eq!(cpu.cpu_scope(), CpuScope::SystemTotal);
            assert!(cpu.sample_window_ms().unwrap_or(0) >= 100);
            assert!(
                cpu.usage_percent()
                    .value()
                    .is_some_and(|v| (0.0..=100.0).contains(v))
            );
        } else {
            // Kernel counter discontinuity (including iowait regression) is a valid refusal.
            assert_eq!(cpu.usage_percent().status(), TelemetryStatus::Error);
            assert!(cpu.usage_percent().value().is_none());
        }
    }
}
