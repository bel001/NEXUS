#![forbid(unsafe_code)]

//! Integración del proveedor nativo y serialización del inventario.

use adaptive_domain::HardwareInventory;
use adaptive_hardware_discovery::{HardwareProvider, capture};

pub fn inventory_from(provider: &impl HardwareProvider) -> HardwareInventory {
    capture(provider)
}

#[cfg(target_os = "linux")]
pub fn native_inventory() -> HardwareInventory {
    inventory_from(&adaptive_platform_linux::LinuxHardwareProvider::new())
}

#[cfg(target_os = "windows")]
pub fn native_inventory() -> HardwareInventory {
    inventory_from(&adaptive_platform_windows::WindowsHardwareProvider::new())
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn native_inventory() -> HardwareInventory {
    HardwareInventory::empty()
}

pub fn to_json(inventory: &HardwareInventory) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(inventory)
}

pub fn run() -> Result<String, serde_json::Error> {
    to_json(&native_inventory())
}

/// B2: one opt-in CPU measurement; memory and energy remain uncollected.
pub fn cpu_telemetry_json() -> Result<String, serde_json::Error> {
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    use adaptive_domain::telemetry::{CpuScope, CpuUsageReading};
    use adaptive_domain::telemetry::{
        PowerSource, TelemetryReading, TelemetryReason, TelemetrySnapshot, TelemetrySource,
    };
    #[cfg(target_os = "linux")]
    let cpu = adaptive_platform_linux::cpu_telemetry::collect_cpu_telemetry();
    #[cfg(target_os = "windows")]
    let cpu = adaptive_platform_windows::cpu_telemetry::collect_cpu_telemetry();
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    let cpu = CpuUsageReading::unavailable(
        TelemetrySource::Orchestrator,
        CpuScope::Unknown,
        TelemetryReason::NotCollected,
    );
    let source = TelemetrySource::Orchestrator;
    let absent_memory =
        || TelemetryReading::<u64>::unavailable(source, TelemetryReason::NotCollected);
    let power = TelemetryReading::<PowerSource>::unavailable(source, TelemetryReason::NotCollected);
    let battery = TelemetryReading::<u8>::unavailable(source, TelemetryReason::NotCollected);
    let snapshot = TelemetrySnapshot::new(
        cpu,
        absent_memory(),
        absent_memory(),
        power,
        battery,
        vec![],
    )
    .expect("uncollected metrics satisfy snapshot invariants");
    serde_json::to_string_pretty(&snapshot)
}

/// B1: a single native memory snapshot, leaving other metrics uncollected.
pub fn memory_telemetry_json() -> Result<String, serde_json::Error> {
    use adaptive_domain::telemetry::{
        CpuScope, CpuUsageReading, PowerSource, TelemetryReading, TelemetryReason,
        TelemetrySnapshot, TelemetrySource,
    };
    #[cfg(target_os = "linux")]
    let (total, available) = adaptive_platform_linux::memory_telemetry::collect_memory_telemetry();
    #[cfg(target_os = "windows")]
    let (total, available) =
        adaptive_platform_windows::memory_telemetry::collect_memory_telemetry();
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    let (total, available) = (
        TelemetryReading::<u64>::unsupported(
            TelemetrySource::Orchestrator,
            TelemetryReason::NotCollected,
        ),
        TelemetryReading::<u64>::unsupported(
            TelemetrySource::Orchestrator,
            TelemetryReason::NotCollected,
        ),
    );
    let source = TelemetrySource::Orchestrator;
    let cpu =
        CpuUsageReading::unavailable(source, CpuScope::Unknown, TelemetryReason::NotCollected);
    let power = TelemetryReading::<PowerSource>::unavailable(source, TelemetryReason::NotCollected);
    let battery = TelemetryReading::<u8>::unavailable(source, TelemetryReason::NotCollected);
    let snapshot = TelemetrySnapshot::new(
        cpu.clone(),
        total,
        available,
        power.clone(),
        battery.clone(),
        vec![],
    )
    .unwrap_or_else(|_| {
        let total = TelemetryReading::<u64>::error(source, TelemetryReason::InvalidMemory);
        let available = TelemetryReading::<u64>::error(source, TelemetryReason::InvalidMemory);
        TelemetrySnapshot::new(cpu, total, available, power, battery, vec![])
            .expect("all absent readings satisfy snapshot invariants")
    });
    serde_json::to_string_pretty(&snapshot)
}

#[cfg(test)]
mod tests {
    use adaptive_domain::{
        DataSource, HardwareComponent, HardwareError, HardwareErrorKind, HardwareInventory,
    };

    use super::*;

    struct EmptyProvider;

    impl HardwareProvider for EmptyProvider {
        fn collect(&self) -> Result<HardwareInventory, HardwareError> {
            Ok(HardwareInventory::empty())
        }
    }

    #[test]
    fn output_is_valid_versioned_json_without_redundant_timestamps() {
        let inventory = inventory_from(&EmptyProvider);
        let json = to_json(&inventory).expect("serializable inventory");
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(value["schema_version"], 1);
        assert!(value.get("captured_at").is_some());
        assert!(value["cpu"].get("timestamp").is_none());
        assert_eq!(inventory.cpu.source(), DataSource::Orchestrator);
    }

    #[test]
    fn simulated_windows_inventory_preserves_structured_incidents_in_json() {
        let mut inventory = HardwareInventory::empty();
        inventory.issues.push(HardwareError::new(
            HardwareComponent::Storage,
            HardwareErrorKind::InvalidData,
            DataSource::WindowsWmiStorage,
            "unrecognized storage code",
        ));
        let json = to_json(&inventory).expect("serializable inventory");
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(value["issues"][0]["source"], "windows_wmi_storage");
        assert_eq!(value["issues"][0]["kind"], "invalid_data");
        assert!(value["cpu"].get("timestamp").is_none());
    }
}
