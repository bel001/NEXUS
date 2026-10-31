use adaptive_domain::{
    Architecture, BatteryInfo, CapabilityInfo, CapabilityKind, CpuInfo, DataSource, GpuInfo,
    HardwareComponent, HardwareError, HardwareErrorKind, HardwareInventory, MemoryInfo,
    Observation, OperatingSystemInfo, OsBitness, StorageDevice, StorageMediumTechnology,
    StorageNature, SystemManufacturerInfo, Unit, architecture_from_rust_name,
};
use adaptive_hardware_discovery::HardwareProvider;
use serde::Deserialize;
use wmi::WMIConnection;

use crate::normalization::{
    CpuArchitectureIssue, CpuRecord, PrimaryStorageDecision, aggregate_cpu, battery_status,
    physical_disk_bus_type, physical_disk_media_type, primary_storage_decision, spindle_speed,
    storage_evidence_is_contradictory, supports_removable_media, visible_memory_bytes,
};
use crate::queries::{
    BATTERY_QUERY, COMPUTER_QUERY, CPU_QUERY, DISK_DRIVE_QUERY, GPU_QUERY, OS_QUERY,
    PHYSICAL_DISK_QUERY,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct WindowsHardwareProvider;

impl WindowsHardwareProvider {
    pub fn new() -> Self {
        Self
    }
}

impl HardwareProvider for WindowsHardwareProvider {
    fn collect(&self) -> Result<HardwareInventory, HardwareError> {
        std::thread::spawn(collect_on_wmi_thread)
            .join()
            .map_err(|_| {
                HardwareError::new(
                    HardwareComponent::Provider,
                    HardwareErrorKind::ProviderFailure,
                    DataSource::WindowsWmiCimv2,
                    "WMI worker thread terminated unexpectedly",
                )
            })
    }
}

fn collect_on_wmi_thread() -> HardwareInventory {
    let mut inventory = HardwareInventory::empty();
    match WMIConnection::new() {
        Ok(connection) => collect_cimv2(&connection, &mut inventory),
        Err(error) => {
            inventory.operating_system =
                Observation::error(DataSource::WindowsWmiCimv2, "CIMV2 connection failed");
            inventory.system_manufacturer =
                Observation::error(DataSource::WindowsWmiCimv2, "CIMV2 connection failed");
            inventory.cpu =
                Observation::error(DataSource::WindowsWmiCimv2, "CIMV2 connection failed");
            inventory.memory =
                Observation::error(DataSource::WindowsWmiCimv2, "CIMV2 connection failed");
            inventory.gpus =
                Observation::error(DataSource::WindowsWmiCimv2, "CIMV2 connection failed");
            inventory.batteries =
                Observation::error(DataSource::WindowsWmiCimv2, "CIMV2 connection failed");
            inventory.issues.push(wmi_issue(
                HardwareComponent::Provider,
                "CIMV2 connection failed",
                &error,
                DataSource::WindowsWmiCimv2,
            ));
        }
    }
    collect_storage(&mut inventory);
    inventory.capabilities = Observation::available(
        vec![CapabilityInfo {
            kind: CapabilityKind::HardwareDiscovery,
            available: Observation::available(true, DataSource::WindowsWmiCimv2, None),
        }],
        DataSource::WindowsWmiCimv2,
        None,
    );
    inventory
}

fn collect_cimv2(connection: &WMIConnection, inventory: &mut HardwareInventory) {
    match connection.raw_query::<Win32OperatingSystem>(OS_QUERY) {
        Ok(rows) => {
            if let Some(row) = rows.into_iter().next() {
                inventory.operating_system = os_from_row(&row);
                inventory.memory = memory_from_row(&row);
            } else {
                inventory.operating_system = Observation::unavailable(
                    DataSource::WindowsWmiCimv2,
                    "operating-system query returned no rows",
                );
                inventory.memory = Observation::unavailable(
                    DataSource::WindowsWmiCimv2,
                    "operating-system query returned no memory row",
                );
            }
        }
        Err(error) => {
            inventory.operating_system =
                Observation::error(DataSource::WindowsWmiCimv2, "operating-system query failed");
            inventory.memory = Observation::error(
                DataSource::WindowsWmiCimv2,
                "operating-system memory query failed",
            );
            inventory.issues.push(wmi_issue(
                HardwareComponent::OperatingSystem,
                "operating-system query failed",
                &error,
                DataSource::WindowsWmiCimv2,
            ));
        }
    }

    match connection.raw_query::<Win32ComputerSystem>(COMPUTER_QUERY) {
        Ok(rows) => {
            if let Some(row) = rows.into_iter().next() {
                inventory.system_manufacturer = Observation::available(
                    SystemManufacturerInfo {
                        manufacturer: optional_string(
                            row.manufacturer,
                            DataSource::WindowsWmiCimv2,
                            "manufacturer is unavailable",
                        ),
                        model: optional_string(
                            row.model,
                            DataSource::WindowsWmiCimv2,
                            "model is unavailable",
                        ),
                    },
                    DataSource::WindowsWmiCimv2,
                    None,
                );
            } else {
                inventory.system_manufacturer = Observation::unavailable(
                    DataSource::WindowsWmiCimv2,
                    "computer-system query returned no rows",
                );
            }
        }
        Err(error) => {
            inventory.system_manufacturer =
                Observation::error(DataSource::WindowsWmiCimv2, "computer-system query failed");
            inventory.issues.push(wmi_issue(
                HardwareComponent::SystemManufacturer,
                "computer-system query failed",
                &error,
                DataSource::WindowsWmiCimv2,
            ));
        }
    }

    match connection.raw_query::<Win32Processor>(CPU_QUERY) {
        Ok(rows) => {
            let records = rows
                .into_iter()
                .map(|row| CpuRecord {
                    manufacturer: row.manufacturer,
                    name: row.name,
                    cores: row.number_of_cores,
                    logical_processors: row.number_of_logical_processors,
                    architecture: row.architecture,
                })
                .collect();
            if let Some(cpu) = aggregate_cpu(records) {
                inventory.cpu = cpu_observation(cpu, &mut inventory.issues);
            } else {
                inventory.cpu = Observation::unavailable(
                    DataSource::WindowsWmiCimv2,
                    "processor query returned no rows",
                );
            }
        }
        Err(error) => {
            inventory.cpu =
                Observation::error(DataSource::WindowsWmiCimv2, "processor query failed");
            inventory.issues.push(wmi_issue(
                HardwareComponent::Cpu,
                "processor query failed",
                &error,
                DataSource::WindowsWmiCimv2,
            ));
        }
    }

    match connection.raw_query::<Win32VideoController>(GPU_QUERY) {
        Ok(rows) => {
            let gpus = rows
                .into_iter()
                .map(|row| GpuInfo {
                    manufacturer: optional_string(
                        row.adapter_compatibility,
                        DataSource::WindowsWmiCimv2,
                        "GPU manufacturer is unavailable",
                    ),
                    model: optional_string(
                        row.name,
                        DataSource::WindowsWmiCimv2,
                        "GPU model is unavailable",
                    ),
                })
                .collect();
            inventory.gpus = Observation::available(gpus, DataSource::WindowsWmiCimv2, None);
        }
        Err(error) => {
            inventory.gpus =
                Observation::error(DataSource::WindowsWmiCimv2, "video-controller query failed");
            inventory.issues.push(wmi_issue(
                HardwareComponent::Gpu,
                "video-controller query failed",
                &error,
                DataSource::WindowsWmiCimv2,
            ));
        }
    }

    match connection.raw_query::<Win32Battery>(BATTERY_QUERY) {
        Ok(rows) => {
            let mut batteries = Vec::new();
            let mut partial = false;
            for row in rows {
                let (battery, issue) = battery_from_row(row);
                partial |= issue.is_some();
                inventory.issues.extend(issue);
                batteries.push(battery);
            }
            inventory.batteries = if partial {
                Observation::partial(
                    batteries,
                    DataSource::WindowsWmiCimv2,
                    None,
                    "one or more battery status codes were not recognized",
                )
            } else {
                Observation::available(batteries, DataSource::WindowsWmiCimv2, None)
            };
        }
        Err(error) => {
            inventory.batteries =
                Observation::error(DataSource::WindowsWmiCimv2, "battery query failed");
            inventory.issues.push(wmi_issue(
                HardwareComponent::Battery,
                "battery query failed",
                &error,
                DataSource::WindowsWmiCimv2,
            ));
        }
    }
}

fn battery_from_row(row: Win32Battery) -> (BatteryInfo, Option<HardwareError>) {
    let source = DataSource::WindowsWmiCimv2;
    let (direction, level, issue) = match row.battery_status.map(battery_status) {
        Some(Ok((direction, level))) => (
            Observation::available(direction, source, None),
            Observation::available(level, source, None),
            None,
        ),
        Some(Err(code)) => (
            Observation::available(adaptive_domain::BatteryDirection::Unknown, source, None),
            Observation::available(adaptive_domain::BatteryLevel::Unknown, source, None),
            Some(HardwareError::new(
                HardwareComponent::Battery,
                HardwareErrorKind::InvalidData,
                source,
                format!("unrecognized Win32_Battery.BatteryStatus code {code}"),
            )),
        ),
        None => (
            Observation::unavailable(source, "battery direction is unavailable"),
            Observation::unavailable(source, "battery level is unavailable"),
            None,
        ),
    };
    (
        BatteryInfo {
            percentage: row
                .estimated_charge_remaining
                .and_then(|value| u8::try_from(value).ok())
                .filter(|value| *value <= 100)
                .map(|value| Observation::available(value, source, Some(Unit::Percent)))
                .unwrap_or_else(|| {
                    Observation::unavailable(source, "battery percentage is unavailable")
                }),
            direction,
            level,
        },
        issue,
    )
}

fn collect_storage(inventory: &mut HardwareInventory) {
    collect_storage_with(&mut WmiStorageQueries, inventory);
}

trait StorageQueries {
    fn physical_disks(&mut self) -> Result<Vec<MsftPhysicalDisk>, String>;
    fn disk_drives(&mut self) -> Result<Vec<Win32DiskDrive>, String>;
}

struct WmiStorageQueries;

impl StorageQueries for WmiStorageQueries {
    fn physical_disks(&mut self) -> Result<Vec<MsftPhysicalDisk>, String> {
        let connection = WMIConnection::with_namespace_path("ROOT\\Microsoft\\Windows\\Storage")
            .map_err(|error| error.to_string())?;
        connection
            .raw_query(PHYSICAL_DISK_QUERY)
            .map_err(|error| error.to_string())
    }

    fn disk_drives(&mut self) -> Result<Vec<Win32DiskDrive>, String> {
        let connection = WMIConnection::new().map_err(|error| error.to_string())?;
        connection
            .raw_query(DISK_DRIVE_QUERY)
            .map_err(|error| error.to_string())
    }
}

fn collect_storage_with(queries: &mut impl StorageQueries, inventory: &mut HardwareInventory) {
    let primary_failed = match queries.physical_disks() {
        Ok(rows)
            if primary_storage_decision(Some(rows.len())) == PrimaryStorageDecision::UsePrimary =>
        {
            let mut devices = Vec::new();
            let mut partial = false;
            for row in rows {
                let (device, row_issues) = storage_from_physical_disk(row);
                partial |= !row_issues.is_empty();
                inventory.issues.extend(row_issues);
                devices.push(device);
            }
            inventory.storage = if partial {
                Observation::partial(
                    devices,
                    DataSource::WindowsWmiStorage,
                    None,
                    "one or more physical-disk values were invalid or contradictory",
                )
            } else {
                Observation::available(devices, DataSource::WindowsWmiStorage, None)
            };
            return;
        }
        Ok(_) => false,
        Err(error) => {
            inventory.issues.push(wmi_issue(
                HardwareComponent::Storage,
                "physical-disk source failed; using fallback",
                &error,
                DataSource::WindowsWmiStorage,
            ));
            true
        }
    };

    match queries.disk_drives() {
        Ok(rows) => {
            let devices = rows.into_iter().map(storage_from_disk_drive).collect();
            inventory.storage = if primary_failed {
                Observation::partial(
                    devices,
                    DataSource::WindowsWmiCimv2,
                    None,
                    "the primary physical-disk source failed; fallback data is available",
                )
            } else {
                Observation::available(devices, DataSource::WindowsWmiCimv2, None)
            };
        }
        Err(error) => inventory.issues.push(wmi_issue(
            HardwareComponent::Storage,
            "disk-drive fallback source failed",
            &error,
            DataSource::WindowsWmiCimv2,
        )),
    }

    if inventory.storage.source() == DataSource::Orchestrator {
        inventory.storage = Observation::error(
            DataSource::WindowsWmiCimv2,
            "all approved storage queries failed",
        );
    }
}

fn os_from_row(row: &Win32OperatingSystem) -> Observation<OperatingSystemInfo> {
    let os_architecture = row
        .os_architecture
        .as_deref()
        .map(normalize_os_architecture);
    Observation::available(
        OperatingSystemInfo {
            name: optional_string(
                row.caption.clone(),
                DataSource::WindowsWmiCimv2,
                "OS name is unavailable",
            ),
            version: optional_string(
                row.version.clone(),
                DataSource::WindowsWmiCimv2,
                "OS version is unavailable",
            ),
            distribution: Observation::unsupported(
                DataSource::WindowsWmiCimv2,
                "Linux distribution does not apply to Windows",
            ),
            binary_architecture: Observation::available(
                architecture_from_rust_name(std::env::consts::ARCH),
                DataSource::Runtime,
                None,
            ),
            kernel_architecture: Observation::unavailable(
                DataSource::WindowsWmiCimv2,
                "kernel architecture is not exposed by the selected query",
            ),
            os_bitness: os_architecture
                .map(|(_, bits)| Observation::available(bits, DataSource::WindowsWmiCimv2, None))
                .unwrap_or_else(|| {
                    Observation::unavailable(DataSource::WindowsWmiCimv2, "OS bitness unavailable")
                }),
        },
        DataSource::WindowsWmiCimv2,
        None,
    )
}

fn memory_from_row(row: &Win32OperatingSystem) -> Observation<MemoryInfo> {
    row.total_visible_memory_size
        .and_then(visible_memory_bytes)
        .map(|bytes| {
            Observation::available(
                MemoryInfo {
                    total_visible_bytes: Observation::available(
                        bytes,
                        DataSource::WindowsWmiCimv2,
                        Some(Unit::Bytes),
                    ),
                },
                DataSource::WindowsWmiCimv2,
                None,
            )
        })
        .unwrap_or_else(|| {
            Observation::unavailable(
                DataSource::WindowsWmiCimv2,
                "visible memory is unavailable or overflowed",
            )
        })
}

fn cpu_observation(
    cpu: crate::normalization::AggregatedCpu,
    issues: &mut Vec<HardwareError>,
) -> Observation<CpuInfo> {
    let source = DataSource::WindowsWmiCimv2;
    if let Some(issue) = cpu.architecture_issue {
        let detail = match issue {
            CpuArchitectureIssue::UnknownCode => {
                "one or more processor architecture codes were not recognized"
            }
            CpuArchitectureIssue::ConflictingValues => {
                "processor architecture values are contradictory"
            }
        };
        issues.push(HardwareError::new(
            HardwareComponent::Cpu,
            HardwareErrorKind::InvalidData,
            source,
            detail,
        ));
    }
    let value = CpuInfo {
        manufacturer: optional_string(cpu.manufacturer, source, "CPU manufacturer unavailable"),
        model: optional_string(cpu.name, source, "CPU model unavailable"),
        processor_architecture: cpu
            .architecture
            .map(|value| Observation::available(value, source, None))
            .unwrap_or_else(|| Observation::unavailable(source, "CPU architecture unavailable")),
        package_count: Observation::unavailable(source, "package count is not queried"),
        logical_processor_count: optional_count(
            cpu.logical_processors,
            "logical count unavailable",
        ),
        online_physical_core_count: optional_count(cpu.cores, "core count unavailable"),
    };
    if cpu.duplicate_rows_removed || cpu.architecture_issue.is_some() {
        Observation::partial(
            value,
            source,
            None,
            "processor rows were incomplete, duplicated, or contradictory",
        )
    } else {
        Observation::available(value, source, None)
    }
}

fn storage_from_physical_disk(row: MsftPhysicalDisk) -> (StorageDevice, Vec<HardwareError>) {
    let source = DataSource::WindowsWmiStorage;
    let mut issues = Vec::new();
    let (medium_technology, medium_value) = match row.media_type {
        Some(code) => match physical_disk_media_type(code) {
            Ok(value) => (Observation::available(value, source, None), Some(value)),
            Err(code) => {
                issues.push(HardwareError::new(
                    HardwareComponent::Storage,
                    HardwareErrorKind::InvalidData,
                    source,
                    format!("unrecognized MSFT_PhysicalDisk.MediaType code {code}"),
                ));
                (
                    Observation::available(StorageMediumTechnology::Unknown, source, None),
                    Some(StorageMediumTechnology::Unknown),
                )
            }
        },
        None => (
            Observation::unavailable(source, "media type unavailable"),
            None,
        ),
    };
    let transport = match row.bus_type {
        Some(code) => match physical_disk_bus_type(code) {
            Ok(value) => Observation::available(value, source, None),
            Err(code) => {
                issues.push(HardwareError::new(
                    HardwareComponent::Storage,
                    HardwareErrorKind::InvalidData,
                    source,
                    format!("unrecognized MSFT_PhysicalDisk.BusType code {code}"),
                ));
                Observation::available(adaptive_domain::StorageTransport::Unknown, source, None)
            }
        },
        None => Observation::unavailable(source, "bus type unavailable"),
    };
    let rotation_value = row.spindle_speed.map(spindle_speed);
    if let (Some(medium), Some(rotation)) = (medium_value, rotation_value)
        && storage_evidence_is_contradictory(medium, rotation)
    {
        issues.push(HardwareError::new(
            HardwareComponent::Storage,
            HardwareErrorKind::InvalidData,
            source,
            "MSFT_PhysicalDisk media type contradicts spindle speed",
        ));
    }
    let device = StorageDevice {
        presentation: optional_string(row.friendly_name, source, "disk name unavailable"),
        medium_technology,
        transport,
        nature: Observation::available(StorageNature::Physical, source, None),
        removability: Observation::unavailable(
            source,
            "physical removability is not exposed by the approved query",
        ),
        rotation: rotation_value
            .map(|value| Observation::available(value, source, None))
            .unwrap_or_else(|| Observation::unavailable(source, "spindle speed unavailable")),
        supports_removable_media: Observation::unavailable(
            source,
            "physical-disk query does not prove removable-media support",
        ),
        capacity_bytes: optional_bytes(row.size, source, "capacity unavailable"),
        reported_sector_size_bytes: Observation::unavailable(
            source,
            "no generic reported sector value",
        ),
        logical_sector_size_bytes: optional_sector(
            row.logical_sector_size,
            source,
            "logical sector size unavailable",
        ),
        physical_sector_size_bytes: optional_sector(
            row.physical_sector_size,
            source,
            "physical sector size unavailable",
        ),
    };
    (device, issues)
}

fn storage_from_disk_drive(row: Win32DiskDrive) -> StorageDevice {
    let source = DataSource::WindowsWmiCimv2;
    let supports_removable = supports_removable_media(row.capabilities.as_deref());
    let medium_detail = if row
        .media_type
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        "Win32_DiskDrive MediaType is present but is not a reliable medium technology"
    } else {
        "Win32_DiskDrive MediaType is unavailable"
    };
    StorageDevice {
        presentation: optional_string(
            row.model.or(row.manufacturer),
            source,
            "disk presentation unavailable",
        ),
        medium_technology: Observation::unavailable(source, medium_detail),
        transport: Observation::unavailable(source, "transport unavailable in fallback"),
        nature: Observation::available(StorageNature::Physical, source, None),
        removability: Observation::unavailable(
            source,
            "physical removability is not exposed by the fallback query",
        ),
        rotation: Observation::unavailable(source, "rotation unavailable in fallback"),
        supports_removable_media: supports_removable
            .map(|value| Observation::available(value, source, None))
            .unwrap_or_else(|| {
                Observation::unavailable(source, "removable-media capability unavailable")
            }),
        capacity_bytes: optional_bytes(row.size, source, "capacity unavailable"),
        reported_sector_size_bytes: optional_sector(
            row.bytes_per_sector,
            source,
            "reported sector size unavailable",
        ),
        logical_sector_size_bytes: Observation::unavailable(
            source,
            "logical sector size unavailable in fallback",
        ),
        physical_sector_size_bytes: Observation::unavailable(
            source,
            "physical sector size unavailable in fallback",
        ),
    }
}

fn optional_string(value: Option<String>, source: DataSource, detail: &str) -> Observation<String> {
    value
        .filter(|value| !value.trim().is_empty())
        .map(|value| Observation::available(value, source, None))
        .unwrap_or_else(|| Observation::unavailable(source, detail))
}

fn optional_count(value: Option<u32>, detail: &str) -> Observation<u32> {
    value
        .map(|value| Observation::available(value, DataSource::WindowsWmiCimv2, Some(Unit::Count)))
        .unwrap_or_else(|| Observation::unavailable(DataSource::WindowsWmiCimv2, detail))
}

fn optional_bytes(value: Option<u64>, source: DataSource, detail: &str) -> Observation<u64> {
    value
        .map(|value| Observation::available(value, source, Some(Unit::Bytes)))
        .unwrap_or_else(|| Observation::unavailable(source, detail))
}

fn optional_sector(value: Option<u64>, source: DataSource, detail: &str) -> Observation<u64> {
    value
        .map(|value| Observation::available(value, source, Some(Unit::BytesPerSector)))
        .unwrap_or_else(|| Observation::unavailable(source, detail))
}

fn normalize_os_architecture(value: &str) -> (Architecture, OsBitness) {
    let lower = value.to_ascii_lowercase();
    if lower.contains("64") {
        (Architecture::Unknown, OsBitness::Bits64)
    } else if lower.contains("32") || lower.contains("86") {
        (Architecture::Unknown, OsBitness::Bits32)
    } else {
        (Architecture::Unknown, OsBitness::Unknown)
    }
}

fn wmi_issue(
    component: HardwareComponent,
    detail: &str,
    error: &impl std::fmt::Display,
    source: DataSource,
) -> HardwareError {
    HardwareError::new(
        component,
        HardwareErrorKind::ReadFailure,
        source,
        format!("{detail}: {error}"),
    )
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Win32OperatingSystem {
    caption: Option<String>,
    version: Option<String>,
    #[serde(rename = "OSArchitecture")]
    os_architecture: Option<String>,
    total_visible_memory_size: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Win32ComputerSystem {
    manufacturer: Option<String>,
    model: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Win32Processor {
    manufacturer: Option<String>,
    name: Option<String>,
    number_of_cores: Option<u32>,
    number_of_logical_processors: Option<u32>,
    architecture: Option<u16>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Win32VideoController {
    name: Option<String>,
    adapter_compatibility: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Win32Battery {
    estimated_charge_remaining: Option<u16>,
    battery_status: Option<u16>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct MsftPhysicalDisk {
    friendly_name: Option<String>,
    media_type: Option<u16>,
    bus_type: Option<u16>,
    size: Option<u64>,
    logical_sector_size: Option<u64>,
    physical_sector_size: Option<u64>,
    spindle_speed: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Win32DiskDrive {
    model: Option<String>,
    manufacturer: Option<String>,
    media_type: Option<String>,
    size: Option<u64>,
    bytes_per_sector: Option<u64>,
    capabilities: Option<Vec<u16>>,
}

#[cfg(test)]
mod tests {
    use adaptive_domain::{
        Architecture, ObservationStatus, Rotation, StorageMediumTechnology, StorageTransport,
    };

    use super::*;

    struct FakeStorageQueries {
        primary: Option<Result<Vec<MsftPhysicalDisk>, String>>,
        fallback: Option<Result<Vec<Win32DiskDrive>, String>>,
        fallback_calls: usize,
    }

    impl StorageQueries for FakeStorageQueries {
        fn physical_disks(&mut self) -> Result<Vec<MsftPhysicalDisk>, String> {
            self.primary.take().expect("primary query result")
        }

        fn disk_drives(&mut self) -> Result<Vec<Win32DiskDrive>, String> {
            self.fallback_calls += 1;
            self.fallback.take().expect("fallback query result")
        }
    }

    fn physical_disk() -> MsftPhysicalDisk {
        MsftPhysicalDisk {
            friendly_name: Some("Example primary disk".into()),
            media_type: Some(4),
            bus_type: Some(17),
            size: Some(1024),
            logical_sector_size: Some(512),
            physical_sector_size: Some(4096),
            spindle_speed: Some(0),
        }
    }

    fn fallback_disk() -> Win32DiskDrive {
        Win32DiskDrive {
            model: Some("Example fallback disk".into()),
            manufacturer: None,
            media_type: None,
            size: Some(1024),
            bytes_per_sector: Some(512),
            capabilities: None,
        }
    }

    #[test]
    fn absent_processor_architecture_is_unavailable_without_an_incident() {
        let cpu = aggregate_cpu(vec![CpuRecord {
            manufacturer: None,
            name: None,
            cores: Some(4),
            logical_processors: Some(8),
            architecture: None,
        }])
        .expect("processor row");
        let mut issues = Vec::new();
        let observation = cpu_observation(cpu, &mut issues);
        assert_eq!(
            observation
                .value()
                .expect("CPU")
                .processor_architecture
                .status(),
            ObservationStatus::Unavailable
        );
        assert!(issues.is_empty());
    }

    #[test]
    fn unknown_processor_architecture_is_partial_and_reported() {
        let cpu = aggregate_cpu(vec![CpuRecord {
            manufacturer: None,
            name: None,
            cores: Some(4),
            logical_processors: Some(8),
            architecture: Some(99),
        }])
        .expect("processor row");
        let mut issues = Vec::new();
        let observation = cpu_observation(cpu, &mut issues);
        assert_eq!(observation.status(), ObservationStatus::Partial);
        assert_eq!(
            observation
                .value()
                .expect("CPU")
                .processor_architecture
                .value(),
            Some(&Architecture::Unknown)
        );
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].kind, HardwareErrorKind::InvalidData);
    }

    #[test]
    fn unknown_physical_disk_codes_produce_structured_incidents() {
        let (disk, issues) = storage_from_physical_disk(MsftPhysicalDisk {
            friendly_name: None,
            media_type: Some(99),
            bus_type: Some(99),
            size: None,
            logical_sector_size: None,
            physical_sector_size: None,
            spindle_speed: Some(u32::MAX),
        });
        assert_eq!(
            disk.medium_technology.value(),
            Some(&StorageMediumTechnology::Unknown)
        );
        assert_eq!(disk.transport.value(), Some(&StorageTransport::Unknown));
        assert_eq!(disk.rotation.value(), Some(&Rotation::Unknown));
        assert_eq!(issues.len(), 2);
        assert!(
            issues
                .iter()
                .all(|issue| issue.kind == HardwareErrorKind::InvalidData)
        );
    }

    #[test]
    fn contradictory_storage_evidence_is_retained_and_reported() {
        let (disk, issues) = storage_from_physical_disk(MsftPhysicalDisk {
            friendly_name: Some("Example".into()),
            media_type: Some(3),
            bus_type: Some(17),
            size: Some(1024),
            logical_sector_size: Some(512),
            physical_sector_size: Some(4096),
            spindle_speed: Some(0),
        });
        assert_eq!(
            disk.medium_technology.value(),
            Some(&StorageMediumTechnology::Magnetic)
        );
        assert_eq!(disk.rotation.value(), Some(&Rotation::NonRotational));
        assert!(issues.iter().any(|issue| {
            issue.kind == HardwareErrorKind::InvalidData && issue.detail.contains("contradicts")
        }));
    }

    #[test]
    fn unknown_battery_status_produces_a_structured_incident() {
        let (battery, issue) = battery_from_row(Win32Battery {
            estimated_charge_remaining: Some(50),
            battery_status: Some(99),
        });
        assert_eq!(
            battery.direction.value(),
            Some(&adaptive_domain::BatteryDirection::Unknown)
        );
        assert_eq!(
            battery.level.value(),
            Some(&adaptive_domain::BatteryLevel::Unknown)
        );
        assert_eq!(
            issue.as_ref().map(|issue| issue.kind),
            Some(HardwareErrorKind::InvalidData)
        );
    }

    #[test]
    fn successful_primary_storage_query_does_not_mix_fallback_rows() {
        let mut queries = FakeStorageQueries {
            primary: Some(Ok(vec![physical_disk()])),
            fallback: Some(Ok(vec![fallback_disk()])),
            fallback_calls: 0,
        };
        let mut inventory = HardwareInventory::empty();
        collect_storage_with(&mut queries, &mut inventory);

        assert_eq!(queries.fallback_calls, 0);
        assert_eq!(inventory.storage.source(), DataSource::WindowsWmiStorage);
        assert_eq!(inventory.storage.status(), ObservationStatus::Available);
        assert_eq!(inventory.storage.value().map(Vec::len), Some(1));
    }

    #[test]
    fn empty_primary_storage_query_uses_the_fallback_without_duplication() {
        let mut queries = FakeStorageQueries {
            primary: Some(Ok(Vec::new())),
            fallback: Some(Ok(vec![fallback_disk()])),
            fallback_calls: 0,
        };
        let mut inventory = HardwareInventory::empty();
        collect_storage_with(&mut queries, &mut inventory);

        assert_eq!(queries.fallback_calls, 1);
        assert_eq!(inventory.storage.source(), DataSource::WindowsWmiCimv2);
        assert_eq!(inventory.storage.status(), ObservationStatus::Available);
        assert_eq!(inventory.storage.value().map(Vec::len), Some(1));
        assert!(inventory.issues.is_empty());
    }

    #[test]
    fn failed_or_unavailable_primary_source_preserves_partial_fallback_data() {
        let mut queries = FakeStorageQueries {
            primary: Some(Err("class unavailable or response type mismatch".into())),
            fallback: Some(Ok(vec![fallback_disk()])),
            fallback_calls: 0,
        };
        let mut inventory = HardwareInventory::empty();
        collect_storage_with(&mut queries, &mut inventory);

        assert_eq!(inventory.storage.status(), ObservationStatus::Partial);
        assert_eq!(inventory.storage.value().map(Vec::len), Some(1));
        assert_eq!(inventory.issues.len(), 1);
        assert_eq!(inventory.issues[0].kind, HardwareErrorKind::ReadFailure);
    }

    #[test]
    fn failed_primary_and_fallback_sources_leave_a_structured_error() {
        let mut queries = FakeStorageQueries {
            primary: Some(Err("primary query failed".into())),
            fallback: Some(Err("fallback query failed".into())),
            fallback_calls: 0,
        };
        let mut inventory = HardwareInventory::empty();
        collect_storage_with(&mut queries, &mut inventory);

        assert_eq!(inventory.storage.status(), ObservationStatus::Error);
        assert!(inventory.storage.value().is_none());
        assert_eq!(inventory.issues.len(), 2);
    }

    #[test]
    fn absent_fallback_properties_remain_unavailable() {
        let disk = storage_from_disk_drive(Win32DiskDrive {
            model: None,
            manufacturer: None,
            media_type: None,
            size: None,
            bytes_per_sector: None,
            capabilities: None,
        });
        assert_eq!(disk.presentation.status(), ObservationStatus::Unavailable);
        assert_eq!(disk.capacity_bytes.status(), ObservationStatus::Unavailable);
        assert_eq!(
            disk.supports_removable_media.status(),
            ObservationStatus::Unavailable
        );
    }

    #[test]
    fn native_provider_validates_cpu_battery_and_storage_invariants() {
        let inventory = adaptive_hardware_discovery::capture(&WindowsHardwareProvider::new());
        assert_eq!(
            inventory.schema_version,
            adaptive_domain::HARDWARE_INVENTORY_SCHEMA_VERSION
        );
        assert!(inventory.captured_at.is_some());

        let cpu = inventory
            .cpu
            .value()
            .expect("native WMI CPU query must return a usable component");
        for observation in [&cpu.manufacturer, &cpu.model] {
            assert!(observation.is_consistent());
        }
        assert!(cpu.processor_architecture.is_consistent());
        assert!(cpu.package_count.is_consistent());
        assert!(cpu.logical_processor_count.is_consistent());
        assert!(cpu.online_physical_core_count.is_consistent());

        let batteries = inventory
            .batteries
            .value()
            .expect("native WMI battery query must return a collection, possibly empty");
        for battery in batteries {
            assert!(battery.percentage.is_consistent());
            assert!(battery.direction.is_consistent());
            assert!(battery.level.is_consistent());
        }

        let storage = inventory
            .storage
            .value()
            .expect("native WMI storage query or its fallback must return a collection");
        assert!(
            !storage.is_empty(),
            "native Windows validation requires at least one storage device"
        );
        for device in storage {
            assert!(device.presentation.is_consistent());
            assert!(device.medium_technology.is_consistent());
            assert!(device.transport.is_consistent());
            assert!(device.nature.is_consistent());
            assert!(device.removability.is_consistent());
            assert!(device.rotation.is_consistent());
            assert!(device.supports_removable_media.is_consistent());
            assert!(device.capacity_bytes.is_consistent());
            assert!(device.reported_sector_size_bytes.is_consistent());
            assert!(device.logical_sector_size_bytes.is_consistent());
            assert!(device.physical_sector_size_bytes.is_consistent());
        }
    }
}
