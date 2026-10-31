use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use time::OffsetDateTime;

use crate::{DataSource, Observation, sanitize_detail};

pub const HARDWARE_INVENTORY_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Architecture {
    X86,
    X86_64,
    Arm,
    Arm64,
    Mips,
    Alpha,
    PowerPc,
    Itanium,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OsBitness {
    Bits32,
    Bits64,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageMediumTechnology {
    Magnetic,
    SolidState,
    StorageClassMemory,
    Optical,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageTransport {
    Ata,
    Sata,
    Scsi,
    Sas,
    Nvme,
    Usb,
    Mmc,
    Virtio,
    Pci,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageNature {
    Physical,
    Virtual,
    Raid,
    Loop,
    Ram,
    DeviceMapper,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Removability {
    Removable,
    Fixed,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Rotation {
    Rotational,
    NonRotational,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BatteryDirection {
    Charging,
    Discharging,
    NotCharging,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BatteryLevel {
    Full,
    High,
    Low,
    Critical,
    Partial,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityKind {
    HardwareDiscovery,
    GpuDiscovery,
    StorageDiscovery,
    BatteryDiscovery,
    PowerProfilesDaemon,
    Tlp,
    AutoCpuFreq,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HardwareComponent {
    Provider,
    OperatingSystem,
    SystemManufacturer,
    Cpu,
    Memory,
    Gpu,
    Storage,
    Battery,
    Capability,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HardwareErrorKind {
    Unsupported,
    Unavailable,
    PermissionDenied,
    InvalidData,
    ReadFailure,
    ProviderFailure,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HardwareError {
    pub component: HardwareComponent,
    pub kind: HardwareErrorKind,
    pub source: DataSource,
    pub detail: String,
}

impl HardwareError {
    pub fn new(
        component: HardwareComponent,
        kind: HardwareErrorKind,
        source: DataSource,
        detail: impl AsRef<str>,
    ) -> Self {
        Self {
            component,
            kind,
            source,
            detail: sanitize_detail(detail.as_ref()),
        }
    }
}

impl std::fmt::Display for HardwareError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:?}: {}", self.component, self.detail)
    }
}

impl std::error::Error for HardwareError {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct OperatingSystemInfo {
    pub name: Observation<String>,
    pub version: Observation<String>,
    pub distribution: Observation<String>,
    pub binary_architecture: Observation<Architecture>,
    pub kernel_architecture: Observation<Architecture>,
    pub os_bitness: Observation<OsBitness>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SystemManufacturerInfo {
    pub manufacturer: Observation<String>,
    pub model: Observation<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CpuInfo {
    pub manufacturer: Observation<String>,
    pub model: Observation<String>,
    pub processor_architecture: Observation<Architecture>,
    pub package_count: Observation<u32>,
    pub logical_processor_count: Observation<u32>,
    pub online_physical_core_count: Observation<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MemoryInfo {
    pub total_visible_bytes: Observation<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GpuInfo {
    pub manufacturer: Observation<String>,
    pub model: Observation<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StorageDevice {
    pub presentation: Observation<String>,
    pub medium_technology: Observation<StorageMediumTechnology>,
    pub transport: Observation<StorageTransport>,
    pub nature: Observation<StorageNature>,
    pub removability: Observation<Removability>,
    pub rotation: Observation<Rotation>,
    pub supports_removable_media: Observation<bool>,
    pub capacity_bytes: Observation<u64>,
    pub reported_sector_size_bytes: Observation<u64>,
    pub logical_sector_size_bytes: Observation<u64>,
    pub physical_sector_size_bytes: Observation<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BatteryInfo {
    pub percentage: Observation<u8>,
    pub direction: Observation<BatteryDirection>,
    pub level: Observation<BatteryLevel>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CapabilityInfo {
    pub kind: CapabilityKind,
    pub available: Observation<bool>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HardwareInventory {
    pub schema_version: u32,
    #[serde(with = "time::serde::rfc3339::option")]
    pub captured_at: Option<OffsetDateTime>,
    pub operating_system: Observation<OperatingSystemInfo>,
    pub system_manufacturer: Observation<SystemManufacturerInfo>,
    pub cpu: Observation<CpuInfo>,
    pub memory: Observation<MemoryInfo>,
    pub gpus: Observation<Vec<GpuInfo>>,
    pub storage: Observation<Vec<StorageDevice>>,
    pub batteries: Observation<Vec<BatteryInfo>>,
    pub capabilities: Observation<Vec<CapabilityInfo>>,
    pub issues: Vec<HardwareError>,
}

impl HardwareInventory {
    pub fn empty() -> Self {
        let detail = "provider has not supplied this component";
        Self {
            schema_version: HARDWARE_INVENTORY_SCHEMA_VERSION,
            captured_at: None,
            operating_system: Observation::unavailable(DataSource::Orchestrator, detail),
            system_manufacturer: Observation::unavailable(DataSource::Orchestrator, detail),
            cpu: Observation::unavailable(DataSource::Orchestrator, detail),
            memory: Observation::unavailable(DataSource::Orchestrator, detail),
            gpus: Observation::unavailable(DataSource::Orchestrator, detail),
            storage: Observation::unavailable(DataSource::Orchestrator, detail),
            batteries: Observation::unavailable(DataSource::Orchestrator, detail),
            capabilities: Observation::unavailable(DataSource::Orchestrator, detail),
            issues: Vec::new(),
        }
    }

    pub fn set_captured_at_from_system_time(&mut self, system_time: SystemTime) {
        self.captured_at = system_time
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|duration| i128::try_from(duration.as_nanos()).ok())
            .and_then(|nanoseconds| OffsetDateTime::from_unix_timestamp_nanos(nanoseconds).ok());
    }
}

impl Default for HardwareInventory {
    fn default() -> Self {
        Self::empty()
    }
}

pub fn architecture_from_rust_name(name: &str) -> Architecture {
    match name.to_ascii_lowercase().as_str() {
        "x86" | "i386" | "i586" | "i686" => Architecture::X86,
        "x86_64" | "amd64" => Architecture::X86_64,
        "arm" => Architecture::Arm,
        "aarch64" | "arm64" => Architecture::Arm64,
        "mips" | "mips64" => Architecture::Mips,
        "alpha" => Architecture::Alpha,
        "powerpc" | "powerpc64" | "ppc" | "ppc64" => Architecture::PowerPc,
        "ia64" => Architecture::Itanium,
        _ => Architecture::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captured_at_is_common_and_optional() {
        let mut inventory = HardwareInventory::empty();
        assert!(inventory.captured_at.is_none());
        inventory.set_captured_at_from_system_time(UNIX_EPOCH);
        assert_eq!(inventory.captured_at, Some(OffsetDateTime::UNIX_EPOCH));
    }

    #[test]
    fn unknown_architecture_is_not_guessed() {
        assert_eq!(
            architecture_from_rust_name("mystery"),
            Architecture::Unknown
        );
    }
}
