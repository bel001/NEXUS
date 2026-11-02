#![forbid(unsafe_code)]

//! Adaptador Linux read-only basado en `/proc`, sysfs, `os-release` y uname.

pub mod memory_telemetry;
mod parsers;
mod power_profiles;
mod storage;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use adaptive_domain::{
    Architecture, BatteryDirection, BatteryInfo, BatteryLevel, CapabilityInfo, CapabilityKind,
    CpuInfo, DataSource, GpuInfo, HardwareComponent, HardwareError, HardwareErrorKind,
    HardwareInventory, MemoryInfo, Observation, OperatingSystemInfo, OsBitness,
    SystemManufacturerInfo, Unit, architecture_from_rust_name,
};
use adaptive_hardware_discovery::HardwareProvider;
use parsers::{
    parse_cpu_list, parse_cpuinfo, parse_mem_total_bytes, parse_os_release, topology_counts,
};

/// Proveedor Linux con raíz inyectable para fixtures.
#[derive(Clone, Debug)]
pub struct LinuxHardwareProvider {
    root: PathBuf,
}

impl LinuxHardwareProvider {
    pub fn new() -> Self {
        Self {
            root: PathBuf::from("/"),
        }
    }

    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative.trim_start_matches('/'))
    }
}

impl Default for LinuxHardwareProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl HardwareProvider for LinuxHardwareProvider {
    fn collect(&self) -> Result<HardwareInventory, HardwareError> {
        let mut inventory = HardwareInventory::empty();
        inventory.operating_system = self.collect_operating_system(&mut inventory.issues);
        inventory.system_manufacturer = self.collect_system_manufacturer(&mut inventory.issues);
        inventory.cpu = self.collect_cpu(&mut inventory.issues);
        inventory.memory = self.collect_memory(&mut inventory.issues);
        inventory.gpus = self.collect_gpus(&mut inventory.issues);
        let storage = storage::collect_storage(&self.root);
        inventory.storage = storage.devices;
        inventory.issues.extend(storage.issues);
        inventory.batteries = self.collect_batteries(&mut inventory.issues);
        inventory.capabilities = self.collect_capabilities(&mut inventory.issues);
        Ok(inventory)
    }
}

impl LinuxHardwareProvider {
    fn collect_operating_system(
        &self,
        issues: &mut Vec<HardwareError>,
    ) -> Observation<OperatingSystemInfo> {
        let release = fs::read_to_string(self.path("etc/os-release"))
            .map(|content| parse_os_release(&content));
        if let Err(error) = &release {
            issues.push(io_issue(
                HardwareComponent::OperatingSystem,
                DataSource::LinuxOsRelease,
                "os-release could not be read",
                error,
            ));
        }
        let values = release.as_ref().ok();
        let uname = uname_values();
        let kernel_architecture = uname
            .as_ref()
            .map(|(_, _, machine)| architecture_from_rust_name(machine));
        let os_bitness = kernel_architecture.map(|architecture| match architecture {
            Architecture::X86_64 | Architecture::Arm64 | Architecture::PowerPc => OsBitness::Bits64,
            Architecture::X86 | Architecture::Arm => OsBitness::Bits32,
            _ => OsBitness::Unknown,
        });
        let info = OperatingSystemInfo {
            name: uname
                .as_ref()
                .map(|(name, _, _)| {
                    Observation::available(name.clone(), DataSource::LinuxUname, None)
                })
                .unwrap_or_else(|| {
                    Observation::unavailable(DataSource::LinuxUname, "uname is unavailable")
                }),
            version: values
                .as_ref()
                .and_then(|values| values.get("VERSION_ID").cloned())
                .map(|value| Observation::available(value, DataSource::LinuxOsRelease, None))
                .or_else(|| {
                    uname.as_ref().map(|(_, release, _)| {
                        Observation::available(release.clone(), DataSource::LinuxUname, None)
                    })
                })
                .unwrap_or_else(|| {
                    Observation::unavailable(DataSource::LinuxUname, "OS version is unavailable")
                }),
            distribution: values
                .as_ref()
                .and_then(|values| {
                    values
                        .get("PRETTY_NAME")
                        .or_else(|| values.get("NAME"))
                        .cloned()
                })
                .map(|value| Observation::available(value, DataSource::LinuxOsRelease, None))
                .unwrap_or_else(|| match &release {
                    Err(error) => io_missing(
                        DataSource::LinuxOsRelease,
                        "distribution is unavailable",
                        error,
                    ),
                    Ok(_) => Observation::unavailable(
                        DataSource::LinuxOsRelease,
                        "distribution is unavailable",
                    ),
                }),
            binary_architecture: Observation::available(
                architecture_from_rust_name(std::env::consts::ARCH),
                DataSource::Runtime,
                None,
            ),
            kernel_architecture: kernel_architecture
                .map(|value| Observation::available(value, DataSource::LinuxUname, None))
                .unwrap_or_else(|| {
                    Observation::unavailable(
                        DataSource::LinuxUname,
                        "kernel architecture is unavailable",
                    )
                }),
            os_bitness: os_bitness
                .map(|value| Observation::available(value, DataSource::LinuxUname, None))
                .unwrap_or_else(|| {
                    Observation::unavailable(DataSource::LinuxUname, "OS bitness is unavailable")
                }),
        };
        wrap_component(
            info,
            DataSource::LinuxUname,
            issues,
            HardwareComponent::OperatingSystem,
        )
    }

    fn collect_system_manufacturer(
        &self,
        issues: &mut Vec<HardwareError>,
    ) -> Observation<SystemManufacturerInfo> {
        let manufacturer = read_trimmed(self.path("sys/class/dmi/id/sys_vendor"));
        let model = read_trimmed(self.path("sys/class/dmi/id/product_name"));
        if let Err(error) = &manufacturer {
            issues.push(io_issue(
                HardwareComponent::SystemManufacturer,
                DataSource::LinuxDmi,
                "system manufacturer is unavailable",
                error,
            ));
        }
        if let Err(error) = &model {
            issues.push(io_issue(
                HardwareComponent::SystemManufacturer,
                DataSource::LinuxDmi,
                "system model is unavailable",
                error,
            ));
        }
        let info = SystemManufacturerInfo {
            manufacturer: string_observation(manufacturer, DataSource::LinuxDmi, "manufacturer"),
            model: string_observation(model, DataSource::LinuxDmi, "model"),
        };
        wrap_component(
            info,
            DataSource::LinuxDmi,
            issues,
            HardwareComponent::SystemManufacturer,
        )
    }

    fn collect_cpu(&self, issues: &mut Vec<HardwareError>) -> Observation<CpuInfo> {
        let cpuinfo = fs::read_to_string(self.path("proc/cpuinfo"));
        if let Err(error) = &cpuinfo {
            issues.push(io_issue(
                HardwareComponent::Cpu,
                DataSource::LinuxProcfs,
                "cpuinfo could not be read",
                error,
            ));
        }
        let text = cpuinfo.ok().map(|content| parse_cpuinfo(&content));
        let online_text = fs::read_to_string(self.path("sys/devices/system/cpu/online"));
        if let Err(error) = &online_text {
            issues.push(io_issue(
                HardwareComponent::Cpu,
                DataSource::LinuxSysfs,
                "online CPU set could not be read",
                error,
            ));
        }
        let online = online_text
            .as_ref()
            .ok()
            .and_then(|content| parse_cpu_list(content).ok());
        if online_text.is_ok() && online.is_none() {
            issues.push(HardwareError::new(
                HardwareComponent::Cpu,
                HardwareErrorKind::InvalidData,
                DataSource::LinuxSysfs,
                "online CPU set is unavailable or invalid",
            ));
        }
        let topology = online
            .as_ref()
            .map(|online| topology_counts(&self.path("sys/devices/system/cpu"), online))
            .unwrap_or(parsers::TopologyCounts {
                core_count: None,
                package_count: None,
                has_problem: false,
            });
        if topology.has_problem {
            issues.push(HardwareError::new(
                HardwareComponent::Cpu,
                HardwareErrorKind::InvalidData,
                DataSource::LinuxSysfs,
                "CPU topology is incomplete or contradictory",
            ));
        }
        let (logical_count, logical_source) = if let Some(online) = online.as_ref() {
            (u32::try_from(online.len()).ok(), DataSource::LinuxSysfs)
        } else {
            (
                text.as_ref().and_then(|info| info.logical_count),
                DataSource::LinuxProcfs,
            )
        };

        let info = CpuInfo {
            manufacturer: text
                .as_ref()
                .and_then(|info| info.manufacturer.clone())
                .map(|value| Observation::available(value, DataSource::LinuxProcfs, None))
                .unwrap_or_else(|| {
                    Observation::unavailable(
                        DataSource::LinuxProcfs,
                        "CPU manufacturer unavailable",
                    )
                }),
            model: text
                .as_ref()
                .and_then(|info| info.model.clone())
                .map(|value| Observation::available(value, DataSource::LinuxProcfs, None))
                .unwrap_or_else(|| {
                    Observation::unavailable(DataSource::LinuxProcfs, "CPU model unavailable")
                }),
            processor_architecture: Observation::unavailable(
                DataSource::LinuxProcfs,
                "processor architecture is not proven by the selected sources",
            ),
            package_count: count_observation(
                topology.package_count,
                DataSource::LinuxSysfs,
                "package count",
            ),
            logical_processor_count: count_observation(
                logical_count,
                logical_source,
                "logical processor count",
            ),
            online_physical_core_count: count_observation(
                topology.core_count,
                DataSource::LinuxSysfs,
                "online physical core count",
            ),
        };
        wrap_component(info, DataSource::LinuxSysfs, issues, HardwareComponent::Cpu)
    }

    fn collect_memory(&self, issues: &mut Vec<HardwareError>) -> Observation<MemoryInfo> {
        let content = match fs::read_to_string(self.path("proc/meminfo")) {
            Ok(content) => content,
            Err(error) => {
                issues.push(io_issue(
                    HardwareComponent::Memory,
                    DataSource::LinuxProcfs,
                    "meminfo could not be read",
                    &error,
                ));
                return io_missing(DataSource::LinuxProcfs, "meminfo could not be read", &error);
            }
        };
        match parse_mem_total_bytes(&content) {
            Ok(bytes) => Observation::available(
                MemoryInfo {
                    total_visible_bytes: Observation::available(
                        bytes,
                        DataSource::LinuxProcfs,
                        Some(Unit::Bytes),
                    ),
                },
                DataSource::LinuxProcfs,
                None,
            ),
            Err(detail) => {
                issues.push(HardwareError::new(
                    HardwareComponent::Memory,
                    HardwareErrorKind::InvalidData,
                    DataSource::LinuxProcfs,
                    detail,
                ));
                Observation::error(DataSource::LinuxProcfs, detail)
            }
        }
    }

    fn collect_gpus(&self, issues: &mut Vec<HardwareError>) -> Observation<Vec<GpuInfo>> {
        let drm = self.path("sys/class/drm");
        let devices_root = self.path("sys/devices");
        let canonical_devices_root =
            fs::canonicalize(&devices_root).unwrap_or_else(|_| devices_root.clone());
        let entries = match fs::read_dir(drm) {
            Ok(entries) => entries,
            Err(error) => {
                issues.push(io_issue(
                    HardwareComponent::Gpu,
                    DataSource::LinuxSysfs,
                    "DRM class is unavailable",
                    &error,
                ));
                return io_missing(DataSource::LinuxSysfs, "DRM class is unavailable", &error);
            }
        };
        let mut gpus = Vec::new();
        let mut partial = false;
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    partial = true;
                    issues.push(io_issue(
                        HardwareComponent::Gpu,
                        DataSource::LinuxSysfs,
                        "DRM entry could not be read",
                        &error,
                    ));
                    continue;
                }
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.strip_prefix("card").is_some_and(|suffix| {
                !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit())
            }) {
                continue;
            }
            let target = match fs::canonicalize(entry.path()) {
                Ok(target) => target,
                Err(error) => {
                    partial = true;
                    issues.push(io_issue(
                        HardwareComponent::Gpu,
                        DataSource::LinuxSysfs,
                        "DRM link could not be resolved",
                        &error,
                    ));
                    continue;
                }
            };
            if !target.starts_with(&canonical_devices_root) {
                partial = true;
                issues.push(HardwareError::new(
                    HardwareComponent::Gpu,
                    HardwareErrorKind::InvalidData,
                    DataSource::LinuxSysfs,
                    "DRM device resolves outside the device hierarchy",
                ));
                continue;
            }
            let vendor_id = find_ancestor_value(&target, &canonical_devices_root, "vendor");
            let vendor = vendor_id.as_deref().and_then(known_pci_vendor);
            gpus.push(GpuInfo {
                manufacturer: vendor
                    .map(|value| {
                        Observation::available(value.to_owned(), DataSource::LinuxSysfs, None)
                    })
                    .unwrap_or_else(|| {
                        Observation::unavailable(
                            DataSource::LinuxSysfs,
                            "GPU manufacturer is unavailable",
                        )
                    }),
                model: Observation::unavailable(
                    DataSource::LinuxSysfs,
                    "GPU commercial model is unavailable without a trusted mapping",
                ),
            });
        }
        if partial {
            Observation::partial(
                gpus,
                DataSource::LinuxSysfs,
                None,
                "some DRM entries failed",
            )
        } else {
            Observation::available(gpus, DataSource::LinuxSysfs, None)
        }
    }

    fn collect_batteries(&self, issues: &mut Vec<HardwareError>) -> Observation<Vec<BatteryInfo>> {
        let root = self.path("sys/class/power_supply");
        let entries = match fs::read_dir(root) {
            Ok(entries) => entries,
            Err(error) => {
                issues.push(io_issue(
                    HardwareComponent::Battery,
                    DataSource::LinuxSysfs,
                    "power-supply class is unavailable",
                    &error,
                ));
                return io_missing(
                    DataSource::LinuxSysfs,
                    "power-supply class is unavailable",
                    &error,
                );
            }
        };
        let mut batteries = Vec::new();
        let mut partial = false;
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    partial = true;
                    issues.push(io_issue(
                        HardwareComponent::Battery,
                        DataSource::LinuxSysfs,
                        "power-supply entry could not be read",
                        &error,
                    ));
                    continue;
                }
            };
            let target = match fs::canonicalize(entry.path()) {
                Ok(target) => target,
                Err(error) => {
                    partial = true;
                    issues.push(io_issue(
                        HardwareComponent::Battery,
                        DataSource::LinuxSysfs,
                        "power-supply link could not be resolved",
                        &error,
                    ));
                    continue;
                }
            };
            match read_trimmed(target.join("type")) {
                Ok(kind) if kind == "Battery" => {}
                Ok(_) => continue,
                Err(error) => {
                    partial = true;
                    issues.push(io_issue(
                        HardwareComponent::Battery,
                        DataSource::LinuxSysfs,
                        "power-supply type could not be read",
                        &error,
                    ));
                    continue;
                }
            }
            let percentage = match read_trimmed(target.join("capacity")) {
                Ok(value) => match value.parse::<u8>().ok().filter(|value| *value <= 100) {
                    Some(value) => {
                        Observation::available(value, DataSource::LinuxSysfs, Some(Unit::Percent))
                    }
                    None => {
                        partial = true;
                        Observation::error(DataSource::LinuxSysfs, "battery capacity is invalid")
                    }
                },
                Err(error) => {
                    partial = true;
                    io_missing(
                        DataSource::LinuxSysfs,
                        "battery capacity unavailable",
                        &error,
                    )
                }
            };
            let direction = match read_trimmed(target.join("status")) {
                Ok(value) => Observation::available(
                    match value.as_str() {
                        "Charging" => BatteryDirection::Charging,
                        "Discharging" => BatteryDirection::Discharging,
                        "Not charging" | "Full" => BatteryDirection::NotCharging,
                        _ => BatteryDirection::Unknown,
                    },
                    DataSource::LinuxSysfs,
                    None,
                ),
                Err(error) => {
                    partial = true;
                    io_missing(DataSource::LinuxSysfs, "battery status unavailable", &error)
                }
            };
            let level = match read_trimmed(target.join("capacity_level")) {
                Ok(value) => Observation::available(
                    match value.as_str() {
                        "Full" => BatteryLevel::Full,
                        "High" | "Normal" => BatteryLevel::High,
                        "Low" => BatteryLevel::Low,
                        "Critical" => BatteryLevel::Critical,
                        _ => BatteryLevel::Unknown,
                    },
                    DataSource::LinuxSysfs,
                    None,
                ),
                Err(error) => {
                    partial = true;
                    io_missing(DataSource::LinuxSysfs, "battery level unavailable", &error)
                }
            };
            batteries.push(BatteryInfo {
                percentage,
                direction,
                level,
            });
        }
        if partial {
            Observation::partial(
                batteries,
                DataSource::LinuxSysfs,
                None,
                "some battery fields are unavailable",
            )
        } else {
            Observation::available(batteries, DataSource::LinuxSysfs, None)
        }
    }

    fn collect_capabilities(
        &self,
        issues: &mut Vec<HardwareError>,
    ) -> Observation<Vec<CapabilityInfo>> {
        let mut capabilities = vec![CapabilityInfo {
            kind: CapabilityKind::HardwareDiscovery,
            available: Observation::available(true, DataSource::LinuxSysfs, None),
        }];
        let (power_profiles, issue) = power_profiles::detect();
        if let Some(issue) = issue {
            issues.push(issue);
        }
        capabilities.push(power_profiles);
        capabilities.extend(
            [CapabilityKind::Tlp, CapabilityKind::AutoCpuFreq].map(|kind| CapabilityInfo {
                kind,
                available: Observation::unavailable(
                    DataSource::LinuxSysfs,
                    "no approved read-only source proves whether this manager is active",
                ),
            }),
        );
        Observation::available(capabilities, DataSource::LinuxDbus, None)
    }
}

fn wrap_component<T>(
    value: T,
    source: DataSource,
    issues: &[HardwareError],
    component: HardwareComponent,
) -> Observation<T> {
    if issues.iter().any(|issue| issue.component == component) {
        Observation::partial(value, source, None, "some component fields are unavailable")
    } else {
        Observation::available(value, source, None)
    }
}

fn string_observation(
    result: io::Result<String>,
    source: DataSource,
    label: &str,
) -> Observation<String> {
    match result {
        Ok(value) if !value.is_empty() => Observation::available(value, source, None),
        Ok(_) => Observation::unavailable(source, format!("{label} is empty")),
        Err(error) => io_missing(source, format!("{label} is unavailable"), &error),
    }
}

fn count_observation(value: Option<u32>, source: DataSource, label: &str) -> Observation<u32> {
    value
        .map(|value| Observation::available(value, source, Some(Unit::Count)))
        .unwrap_or_else(|| Observation::unavailable(source, format!("{label} is unavailable")))
}

fn read_trimmed(path: impl AsRef<Path>) -> io::Result<String> {
    fs::read_to_string(path).map(|value| value.trim().to_owned())
}

fn io_missing<T>(source: DataSource, detail: impl AsRef<str>, error: &io::Error) -> Observation<T> {
    match error.kind() {
        io::ErrorKind::NotFound => Observation::unavailable(source, detail),
        io::ErrorKind::PermissionDenied => Observation::permission_denied(source, detail),
        _ => Observation::error(source, detail),
    }
}

fn io_issue(
    component: HardwareComponent,
    source: DataSource,
    detail: &str,
    error: &io::Error,
) -> HardwareError {
    let kind = if error.kind() == io::ErrorKind::PermissionDenied {
        HardwareErrorKind::PermissionDenied
    } else {
        HardwareErrorKind::ReadFailure
    };
    HardwareError::new(component, kind, source, detail)
}

fn find_ancestor_value(start: &Path, stop: &Path, attribute: &str) -> Option<String> {
    let mut current = Some(start);
    while let Some(path) = current {
        if !path.starts_with(stop) {
            break;
        }
        if let Ok(value) = read_trimmed(path.join(attribute))
            && !value.is_empty()
        {
            return Some(value);
        }
        if path == stop {
            break;
        }
        current = path.parent();
    }
    None
}

fn known_pci_vendor(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "0x8086" => Some("Intel"),
        "0x1002" => Some("AMD"),
        "0x10de" => Some("NVIDIA"),
        _ => None,
    }
}

#[cfg(target_os = "linux")]
fn uname_values() -> Option<(String, String, String)> {
    let uname = rustix::system::uname();
    Some((
        uname.sysname().to_string_lossy().into_owned(),
        uname.release().to_string_lossy().into_owned(),
        uname.machine().to_string_lossy().into_owned(),
    ))
}

#[cfg(not(target_os = "linux"))]
fn uname_values() -> Option<(String, String, String)> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_gpu_vendors_are_not_free_form_guesses() {
        assert_eq!(known_pci_vendor("0x8086"), Some("Intel"));
        assert_eq!(known_pci_vendor("0xffff"), None);
    }

    #[test]
    fn empty_battery_directory_is_available() {
        let temp = tempfile::tempdir().expect("temporary fixture");
        fs::create_dir_all(temp.path().join("sys/class/power_supply")).expect("fixture directory");
        let provider = LinuxHardwareProvider::with_root(temp.path());
        let observation = provider.collect_batteries(&mut Vec::new());
        assert_eq!(
            observation.status(),
            adaptive_domain::ObservationStatus::Available
        );
        assert_eq!(observation.value().map(Vec::len), Some(0));
    }

    #[test]
    fn permission_denied_is_not_collapsed_into_unavailable() {
        let error = io::Error::new(io::ErrorKind::PermissionDenied, "fixture");
        let observation = io_missing::<String>(DataSource::LinuxSysfs, "denied", &error);
        assert_eq!(
            observation.status(),
            adaptive_domain::ObservationStatus::PermissionDenied
        );
    }

    #[test]
    fn read_failure_is_not_collapsed_into_unavailable() {
        let error = io::Error::other("fixture");
        let observation = io_missing::<String>(DataSource::LinuxSysfs, "read failed", &error);
        assert_eq!(
            observation.status(),
            adaptive_domain::ObservationStatus::Error
        );
    }
}
