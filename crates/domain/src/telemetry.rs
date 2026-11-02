//! Contrato de telemetría local: únicamente tipos y cálculos puros.
//! La adquisición nativa pertenece a una fase posterior de TASK-002.

use serde::Serialize;
use time::{OffsetDateTime, UtcOffset};

pub const TELEMETRY_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TelemetryStatus {
    Available,
    Unavailable,
    Unsupported,
    PermissionDenied,
    Error,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TelemetrySource {
    Orchestrator,
    Fixture,
    LinuxProcStat,
    LinuxProcMeminfo,
    LinuxPowerSupply,
    WindowsSystemTimes,
    WindowsMemoryStatus,
    WindowsWmiMemory,
    WindowsPowerStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TelemetryUnit {
    Bytes,
    Percent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TelemetryQuality {
    Measured,
    Estimated,
}

/// Códigos predefinidos: no se admiten errores que contengan rutas o identificadores.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TelemetryReason {
    NotCollected,
    FirstSample,
    ReadFailure,
    InvalidCounters,
    InvalidMemory,
    InvalidWindow,
    PartialScope,
    NoBattery,
    PermissionDenied,
    Interrupted,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerSource {
    Ac,
    Battery,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CpuScope {
    SystemTotal,
    ProcessorGroup,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TelemetryValidationError {
    InvalidPercent,
    InvalidWindow,
    PartialCpuScope,
    InvalidMemory,
}

/// No reutiliza Observation<T> ni altera HardwareInventory V1.
#[derive(Clone, Debug, Serialize)]
pub struct TelemetryReading<T> {
    value: Option<T>,
    status: TelemetryStatus,
    source: TelemetrySource,
    unit: Option<TelemetryUnit>,
    #[serde(with = "time::serde::rfc3339::option")]
    measured_at: Option<OffsetDateTime>,
    quality: Option<TelemetryQuality>,
    detail: Option<TelemetryReason>,
}

impl<T> TelemetryReading<T> {
    fn present(
        value: T,
        source: TelemetrySource,
        unit: Option<TelemetryUnit>,
        measured_at: OffsetDateTime,
        quality: TelemetryQuality,
    ) -> Self {
        Self {
            value: Some(value),
            status: TelemetryStatus::Available,
            source,
            unit,
            measured_at: Some(measured_at.to_offset(UtcOffset::UTC)),
            quality: Some(quality),
            detail: None,
        }
    }

    fn absent(status: TelemetryStatus, source: TelemetrySource, reason: TelemetryReason) -> Self {
        Self {
            value: None,
            status,
            source,
            unit: None,
            measured_at: None,
            quality: None,
            detail: Some(reason),
        }
    }

    pub fn unavailable(source: TelemetrySource, reason: TelemetryReason) -> Self {
        Self::absent(TelemetryStatus::Unavailable, source, reason)
    }

    pub fn unsupported(source: TelemetrySource, reason: TelemetryReason) -> Self {
        Self::absent(TelemetryStatus::Unsupported, source, reason)
    }

    pub fn permission_denied(source: TelemetrySource) -> Self {
        Self::absent(
            TelemetryStatus::PermissionDenied,
            source,
            TelemetryReason::PermissionDenied,
        )
    }

    pub fn error(source: TelemetrySource, reason: TelemetryReason) -> Self {
        Self::absent(TelemetryStatus::Error, source, reason)
    }

    pub fn value(&self) -> Option<&T> {
        self.value.as_ref()
    }

    pub fn status(&self) -> TelemetryStatus {
        self.status
    }

    pub fn measured_at(&self) -> Option<OffsetDateTime> {
        self.measured_at
    }

    pub fn quality(&self) -> Option<TelemetryQuality> {
        self.quality
    }

    pub fn source(&self) -> TelemetrySource {
        self.source
    }

    pub fn detail(&self) -> Option<TelemetryReason> {
        self.detail
    }
}

impl TelemetryReading<f64> {
    pub fn percent(
        value: f64,
        source: TelemetrySource,
        measured_at: OffsetDateTime,
        quality: TelemetryQuality,
    ) -> Result<Self, TelemetryValidationError> {
        if !value.is_finite() || !(0.0..=100.0).contains(&value) {
            return Err(TelemetryValidationError::InvalidPercent);
        }
        Ok(Self::present(
            value,
            source,
            Some(TelemetryUnit::Percent),
            measured_at,
            quality,
        ))
    }
}

impl TelemetryReading<u8> {
    pub fn percent(
        value: u8,
        source: TelemetrySource,
        measured_at: OffsetDateTime,
        quality: TelemetryQuality,
    ) -> Result<Self, TelemetryValidationError> {
        if value > 100 {
            return Err(TelemetryValidationError::InvalidPercent);
        }
        Ok(Self::present(
            value,
            source,
            Some(TelemetryUnit::Percent),
            measured_at,
            quality,
        ))
    }
}

impl TelemetryReading<u64> {
    pub fn bytes_now(value: u64, source: TelemetrySource, quality: TelemetryQuality) -> Self {
        Self::bytes(value, source, OffsetDateTime::now_utc(), quality)
    }

    pub fn bytes(
        value: u64,
        source: TelemetrySource,
        measured_at: OffsetDateTime,
        quality: TelemetryQuality,
    ) -> Self {
        Self::present(
            value,
            source,
            Some(TelemetryUnit::Bytes),
            measured_at,
            quality,
        )
    }
}

impl TelemetryReading<PowerSource> {
    pub fn power(value: PowerSource, source: TelemetrySource, measured_at: OffsetDateTime) -> Self {
        Self::present(value, source, None, measured_at, TelemetryQuality::Measured)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct CpuUsageReading {
    usage_percent: TelemetryReading<f64>,
    sample_window_ms: Option<u64>,
    cpu_scope: CpuScope,
}

impl CpuUsageReading {
    pub fn usage_percent(&self) -> &TelemetryReading<f64> {
        &self.usage_percent
    }

    pub fn sample_window_ms(&self) -> Option<u64> {
        self.sample_window_ms
    }

    pub fn cpu_scope(&self) -> CpuScope {
        self.cpu_scope
    }
    pub fn available(
        percentage: f64,
        window_ms: u64,
        scope: CpuScope,
        source: TelemetrySource,
        measured_at: OffsetDateTime,
    ) -> Result<Self, TelemetryValidationError> {
        if window_ms == 0 {
            return Err(TelemetryValidationError::InvalidWindow);
        }
        if scope != CpuScope::SystemTotal {
            return Err(TelemetryValidationError::PartialCpuScope);
        }
        Ok(Self {
            usage_percent: TelemetryReading::<f64>::percent(
                percentage,
                source,
                measured_at,
                TelemetryQuality::Estimated,
            )?,
            sample_window_ms: Some(window_ms),
            cpu_scope: scope,
        })
    }

    pub fn unavailable(source: TelemetrySource, scope: CpuScope, reason: TelemetryReason) -> Self {
        Self {
            usage_percent: TelemetryReading::unavailable(source, reason),
            sample_window_ms: None,
            cpu_scope: scope,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TelemetryMetric {
    Cpu,
    Memory,
    Power,
    Battery,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct TelemetryIssue {
    pub metric: TelemetryMetric,
    pub source: TelemetrySource,
    pub reason: TelemetryReason,
}

#[derive(Clone, Debug, Serialize)]
pub struct TelemetrySnapshot {
    schema_version: u32,
    cpu_usage: CpuUsageReading,
    memory_total_visible_bytes: TelemetryReading<u64>,
    memory_available_bytes: TelemetryReading<u64>,
    power_source: TelemetryReading<PowerSource>,
    battery_percent: TelemetryReading<u8>,
    issues: Vec<TelemetryIssue>,
}

impl TelemetrySnapshot {
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn cpu_usage(&self) -> &CpuUsageReading {
        &self.cpu_usage
    }

    pub fn memory_total_visible_bytes(&self) -> &TelemetryReading<u64> {
        &self.memory_total_visible_bytes
    }

    pub fn memory_available_bytes(&self) -> &TelemetryReading<u64> {
        &self.memory_available_bytes
    }

    pub fn power_source(&self) -> &TelemetryReading<PowerSource> {
        &self.power_source
    }

    pub fn battery_percent(&self) -> &TelemetryReading<u8> {
        &self.battery_percent
    }

    pub fn issues(&self) -> &[TelemetryIssue] {
        &self.issues
    }
    pub fn new(
        cpu_usage: CpuUsageReading,
        memory_total_visible_bytes: TelemetryReading<u64>,
        memory_available_bytes: TelemetryReading<u64>,
        power_source: TelemetryReading<PowerSource>,
        battery_percent: TelemetryReading<u8>,
        issues: Vec<TelemetryIssue>,
    ) -> Result<Self, TelemetryValidationError> {
        if let Some(total) = memory_total_visible_bytes.value() {
            if *total == 0 {
                return Err(TelemetryValidationError::InvalidMemory);
            }
            if memory_available_bytes
                .value()
                .is_some_and(|available| available > total)
            {
                return Err(TelemetryValidationError::InvalidMemory);
            }
        }
        Ok(Self {
            schema_version: TELEMETRY_SCHEMA_VERSION,
            cpu_usage,
            memory_total_visible_bytes,
            memory_available_bytes,
            power_source,
            battery_percent,
            issues,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CpuCalculationError {
    Regression,
    Overflow,
    ZeroDelta,
    Inconsistent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LinuxCpuTimes {
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub idle: u64,
    pub iowait: u64,
    pub irq: u64,
    pub softirq: u64,
    pub steal: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowsCpuTimes {
    pub idle: u64,
    pub kernel: u64,
    pub user: u64,
}

fn checked_delta(previous: u64, current: u64) -> Result<u64, CpuCalculationError> {
    current
        .checked_sub(previous)
        .ok_or(CpuCalculationError::Regression)
}

fn percent_from_deltas(total: u64, idle: u64) -> Result<f64, CpuCalculationError> {
    if total == 0 {
        return Err(CpuCalculationError::ZeroDelta);
    }
    if idle > total {
        return Err(CpuCalculationError::Inconsistent);
    }
    Ok(100.0 * (total - idle) as f64 / total as f64)
}

/// Convención conservadora: iowait suma al bucket no ocupado.
pub fn linux_cpu_percent(
    previous: LinuxCpuTimes,
    current: LinuxCpuTimes,
) -> Result<f64, CpuCalculationError> {
    let deltas = [
        checked_delta(previous.user, current.user)?,
        checked_delta(previous.nice, current.nice)?,
        checked_delta(previous.system, current.system)?,
        checked_delta(previous.idle, current.idle)?,
        checked_delta(previous.iowait, current.iowait)?,
        checked_delta(previous.irq, current.irq)?,
        checked_delta(previous.softirq, current.softirq)?,
        checked_delta(previous.steal, current.steal)?,
    ];
    let total = deltas.iter().try_fold(0_u64, |acc, value| {
        acc.checked_add(*value).ok_or(CpuCalculationError::Overflow)
    })?;
    let idle = deltas[3]
        .checked_add(deltas[4])
        .ok_or(CpuCalculationError::Overflow)?;
    percent_from_deltas(total, idle)
}

/// Windows KernelTime incluye IdleTime, por lo que no se suma dos veces.
pub fn windows_cpu_percent(
    previous: WindowsCpuTimes,
    current: WindowsCpuTimes,
) -> Result<f64, CpuCalculationError> {
    let kernel = checked_delta(previous.kernel, current.kernel)?;
    let user = checked_delta(previous.user, current.user)?;
    let idle = checked_delta(previous.idle, current.idle)?;
    if idle > kernel {
        return Err(CpuCalculationError::Inconsistent);
    }
    let total = kernel
        .checked_add(user)
        .ok_or(CpuCalculationError::Overflow)?;
    percent_from_deltas(total, idle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linux() -> LinuxCpuTimes {
        LinuxCpuTimes {
            user: 0,
            nice: 0,
            system: 0,
            idle: 0,
            iowait: 0,
            irq: 0,
            softirq: 0,
            steal: 0,
        }
    }

    #[test]
    fn cpu_linux_two_samples_and_iowait() {
        let start = linux();
        let end = LinuxCpuTimes {
            user: 30,
            system: 20,
            idle: 40,
            iowait: 10,
            ..start
        };
        assert_eq!(linux_cpu_percent(start, end), Ok(50.0));
        assert_eq!(
            linux_cpu_percent(start, start),
            Err(CpuCalculationError::ZeroDelta)
        );
        assert_eq!(
            linux_cpu_percent(end, start),
            Err(CpuCalculationError::Regression)
        );
        let bad = LinuxCpuTimes { iowait: 0, ..end };
        assert_eq!(
            linux_cpu_percent(end, bad),
            Err(CpuCalculationError::Regression)
        );
    }

    #[test]
    fn cpu_linux_detects_overflow() {
        let start = linux();
        let end = LinuxCpuTimes {
            user: u64::MAX,
            system: 1,
            ..start
        };
        assert_eq!(
            linux_cpu_percent(start, end),
            Err(CpuCalculationError::Overflow)
        );
    }

    #[test]
    fn cpu_windows_checked_deltas() {
        let start = WindowsCpuTimes {
            idle: 10,
            kernel: 20,
            user: 30,
        };
        let end = WindowsCpuTimes {
            idle: 30,
            kernel: 70,
            user: 80,
        };
        assert_eq!(windows_cpu_percent(start, end), Ok(80.0));
        assert_eq!(
            windows_cpu_percent(start, start),
            Err(CpuCalculationError::ZeroDelta)
        );
        assert_eq!(
            windows_cpu_percent(end, start),
            Err(CpuCalculationError::Regression)
        );
        let bad = WindowsCpuTimes {
            idle: 80,
            kernel: 70,
            user: 80,
        };
        assert_eq!(
            windows_cpu_percent(start, bad),
            Err(CpuCalculationError::Inconsistent)
        );
        let overflow = WindowsCpuTimes {
            idle: 0,
            kernel: u64::MAX,
            user: 1,
        };
        assert_eq!(
            windows_cpu_percent(
                WindowsCpuTimes {
                    idle: 0,
                    kernel: 0,
                    user: 0
                },
                overflow
            ),
            Err(CpuCalculationError::Overflow)
        );
    }

    #[test]
    fn readings_reject_invalid_percent_and_normalize_time() {
        let instant = OffsetDateTime::UNIX_EPOCH
            .to_offset(UtcOffset::from_hms(5, 0, 0).expect("valid offset"));
        for invalid in [f64::NAN, f64::INFINITY, -1.0, 101.0] {
            assert!(
                TelemetryReading::<f64>::percent(
                    invalid,
                    TelemetrySource::Fixture,
                    instant,
                    TelemetryQuality::Measured
                )
                .is_err()
            );
        }
        assert!(
            TelemetryReading::<u8>::percent(
                255,
                TelemetrySource::Fixture,
                instant,
                TelemetryQuality::Measured
            )
            .is_err()
        );
        let ok = TelemetryReading::<u8>::percent(
            0,
            TelemetrySource::Fixture,
            instant,
            TelemetryQuality::Measured,
        )
        .expect("valid percent");
        assert_eq!(ok.value(), Some(&0));
        assert_eq!(
            ok.measured_at().expect("timestamp").offset(),
            UtcOffset::UTC
        );
        let missing = TelemetryReading::<u8>::unsupported(
            TelemetrySource::Fixture,
            TelemetryReason::NoBattery,
        );
        assert_eq!(missing.value(), None);
        assert_eq!(missing.quality(), None);
        assert_eq!(missing.measured_at(), None);
    }

    #[test]
    fn cpu_window_requires_global_scope_and_positive_duration() {
        let when = OffsetDateTime::UNIX_EPOCH;
        assert!(
            CpuUsageReading::available(
                33.0,
                0,
                CpuScope::SystemTotal,
                TelemetrySource::Fixture,
                when
            )
            .is_err()
        );
        assert!(
            CpuUsageReading::available(
                33.0,
                100,
                CpuScope::ProcessorGroup,
                TelemetrySource::Fixture,
                when
            )
            .is_err()
        );
        let cpu = CpuUsageReading::available(
            33.0,
            100,
            CpuScope::SystemTotal,
            TelemetrySource::Fixture,
            when,
        )
        .expect("valid window");
        assert_eq!(cpu.sample_window_ms, Some(100));
        assert_eq!(cpu.usage_percent.value(), Some(&33.0));
        let first = CpuUsageReading::unavailable(
            TelemetrySource::Fixture,
            CpuScope::Unknown,
            TelemetryReason::FirstSample,
        );
        assert_eq!(first.usage_percent.status(), TelemetryStatus::Unavailable);
        assert!(first.sample_window_ms.is_none());
    }

    #[test]
    fn snapshot_validates_memory_and_serialization_trait() {
        fn assert_serializable<T: Serialize>() {}
        assert_serializable::<TelemetrySnapshot>();
        let when = OffsetDateTime::UNIX_EPOCH;
        let cpu = CpuUsageReading::unavailable(
            TelemetrySource::Fixture,
            CpuScope::Unknown,
            TelemetryReason::FirstSample,
        );
        let total = TelemetryReading::bytes(
            100,
            TelemetrySource::Fixture,
            when,
            TelemetryQuality::Measured,
        );
        let bad_available = TelemetryReading::bytes(
            101,
            TelemetrySource::Fixture,
            when,
            TelemetryQuality::Estimated,
        );
        let power = TelemetryReading::power(PowerSource::Ac, TelemetrySource::Fixture, when);
        let battery =
            TelemetryReading::unsupported(TelemetrySource::Fixture, TelemetryReason::NoBattery);
        assert!(
            TelemetrySnapshot::new(
                cpu.clone(),
                total.clone(),
                bad_available,
                power.clone(),
                battery.clone(),
                vec![]
            )
            .is_err()
        );
        let available = TelemetryReading::bytes(
            50,
            TelemetrySource::Fixture,
            when,
            TelemetryQuality::Estimated,
        );
        let snapshot = TelemetrySnapshot::new(cpu, total, available, power, battery, vec![])
            .expect("valid snapshot");
        assert_eq!(snapshot.schema_version, TELEMETRY_SCHEMA_VERSION);
        assert_eq!(snapshot.memory_available_bytes.value(), Some(&50));
    }
}
