use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use adaptive_domain::{
    DataSource, HardwareComponent, HardwareError, HardwareErrorKind, Observation,
    ObservationStatus, Removability, Rotation, StorageDevice, StorageMediumTechnology,
    StorageNature, StorageTransport, Unit,
};

pub(crate) struct StorageScan {
    pub devices: Observation<Vec<StorageDevice>>,
    pub issues: Vec<HardwareError>,
}

pub(crate) fn collect_storage(root: &Path) -> StorageScan {
    let subsystem_root = root.join("sys/subsystem");
    match fs::metadata(&subsystem_root) {
        Ok(_) => {
            let modern = subsystem_root.join("block/devices");
            return match fs::metadata(&modern) {
                Ok(_) => scan_roots(root, &[modern], DataSource::LinuxSysfsBlockModern, true),
                Err(error) => root_failure(
                    DataSource::LinuxSysfsBlockModern,
                    "modern block subsystem is unavailable",
                    error,
                ),
            };
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => {
            return root_failure(
                DataSource::LinuxSysfsBlockModern,
                "sysfs subsystem classification is unavailable",
                error,
            );
        }
    }

    let candidates = [
        root.join("sys/class/block"),
        root.join("sys/block"),
        root.join("sys/bus/block/devices"),
    ];
    let roots: Vec<PathBuf> = candidates
        .into_iter()
        .filter(|candidate| match fs::metadata(candidate) {
            Ok(_) => true,
            Err(error) => error.kind() != io::ErrorKind::NotFound,
        })
        .collect();
    if roots.is_empty() {
        return StorageScan {
            devices: Observation::unavailable(
                DataSource::LinuxSysfsBlockLegacy,
                "legacy block subsystem is unavailable",
            ),
            issues: Vec::new(),
        };
    }
    scan_roots(root, &roots, DataSource::LinuxSysfsBlockLegacy, false)
}

fn root_failure(source: DataSource, detail: &str, error: io::Error) -> StorageScan {
    let devices = if error.kind() == io::ErrorKind::PermissionDenied {
        Observation::permission_denied(source, detail)
    } else if error.kind() == io::ErrorKind::NotFound {
        Observation::unavailable(source, detail)
    } else {
        Observation::error(source, detail)
    };
    StorageScan {
        devices,
        issues: if error.kind() == io::ErrorKind::NotFound {
            Vec::new()
        } else {
            vec![io_issue(source, detail, &error)]
        },
    }
}

fn scan_roots(root: &Path, roots: &[PathBuf], source: DataSource, modern: bool) -> StorageScan {
    let devices_root = root.join("sys/devices");
    let canonical_devices_root = fs::canonicalize(&devices_root).unwrap_or(devices_root);
    let mut resolved = BTreeMap::<PathBuf, String>::new();
    let mut issues = Vec::new();
    let mut partial = false;
    let mut readable_roots = 0_u32;
    let mut permission_denied = false;

    for classification_root in roots {
        let entries = match fs::read_dir(classification_root) {
            Ok(entries) => entries,
            Err(error) => {
                partial = true;
                permission_denied |= error.kind() == io::ErrorKind::PermissionDenied;
                issues.push(io_issue(
                    source,
                    "block classification could not be read",
                    &error,
                ));
                continue;
            }
        };
        readable_roots += 1;
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    partial = true;
                    issues.push(io_issue(source, "block entry could not be read", &error));
                    continue;
                }
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            match resolve_device(&entry.path(), &canonical_devices_root) {
                Ok(target) => {
                    if target.join("partition").exists() {
                        continue;
                    }
                    resolved.entry(target).or_insert(name);
                }
                Err(detail) => {
                    partial = true;
                    issues.push(HardwareError::new(
                        HardwareComponent::Storage,
                        HardwareErrorKind::InvalidData,
                        source,
                        detail,
                    ));
                }
            }
        }
    }

    if readable_roots == 0 {
        let devices = if permission_denied {
            Observation::permission_denied(source, "block classification access was denied")
        } else {
            Observation::error(source, "block classification could not be read")
        };
        return StorageScan { devices, issues };
    }

    let mut devices = Vec::new();
    for (target, name) in resolved {
        let built = build_storage_device(&name, &target, &canonical_devices_root, source);
        partial |= !built.issues.is_empty();
        issues.extend(built.issues);
        devices.push(built.device);
    }

    let detail = if modern {
        "one or more modern block entries were unavailable"
    } else {
        "one or more legacy block entries were unavailable"
    };
    let observation = if partial {
        Observation::partial(devices, source, None, detail)
    } else {
        Observation::available(devices, source, None)
    };
    StorageScan {
        devices: observation,
        issues,
    }
}

fn resolve_device(entry: &Path, devices_root: &Path) -> Result<PathBuf, &'static str> {
    let target = fs::canonicalize(entry).map_err(|_| "block classification link is broken")?;
    if target.starts_with(devices_root) {
        return Ok(target);
    }

    let recovered = fs::canonicalize(entry.join("device"))
        .map_err(|_| "block entry does not resolve below the device hierarchy")?;
    if recovered.starts_with(devices_root) {
        Ok(recovered)
    } else {
        Err("recovered block entry is outside the device hierarchy")
    }
}

struct BuiltStorageDevice {
    device: StorageDevice,
    issues: Vec<HardwareError>,
}

fn build_storage_device(
    name: &str,
    target: &Path,
    devices_root: &Path,
    source: DataSource,
) -> BuiltStorageDevice {
    let mut issues = Vec::new();
    if let Err(error) = fs::metadata(target) {
        issues.push(device_presence_issue(source, &error));
    }
    let rotation = match read_number(target.join("queue/rotational")) {
        Ok(0) => Ok(Rotation::NonRotational),
        Ok(1) => Ok(Rotation::Rotational),
        Ok(_) => Err(AttributeError::Invalid),
        Err(error) => Err(error),
    };
    let rotation_observation = match rotation {
        Ok(value) => Observation::available(value, source, None),
        Err(error) => attribute_failure(source, "rotation is unavailable", error),
    };
    if let Err(error) = rotation {
        push_attribute_issue(&mut issues, source, "rotation could not be read", error);
    }
    let medium_technology = match rotation {
        Ok(Rotation::Rotational) => {
            Observation::available(StorageMediumTechnology::Magnetic, source, None)
        }
        _ => Observation::unavailable(source, "non-rotational does not prove a storage technology"),
    };

    let capacity_result = read_number(target.join("size"));
    let capacity_bytes = match capacity_result {
        Ok(sectors) => sectors
            .checked_mul(512)
            .map(|bytes| Observation::available(bytes, source, Some(Unit::Bytes)))
            .unwrap_or_else(|| {
                issues.push(HardwareError::new(
                    HardwareComponent::Storage,
                    HardwareErrorKind::InvalidData,
                    source,
                    "storage capacity overflows bytes",
                ));
                Observation::error(source, "capacity overflows bytes")
            }),
        Err(error) => attribute_failure(source, "capacity is unavailable", error),
    };
    if let Err(error) = capacity_result {
        push_attribute_issue(&mut issues, source, "capacity could not be read", error);
    }

    let removability = physical_removability(target, devices_root, source);
    push_observation_issue(
        &mut issues,
        source,
        "physical removability could not be read",
        &removability,
    );
    let removable_media_result = read_bool(target.join("removable"));
    let supports_removable_media = match removable_media_result {
        Ok(value) => Observation::available(value, source, None),
        Err(error) => attribute_failure(source, "removable-media support is unavailable", error),
    };
    if let Err(error) = removable_media_result {
        push_attribute_issue(
            &mut issues,
            source,
            "removable-media support could not be read",
            error,
        );
    }
    let logical_sector_size_bytes = byte_observation(
        target.join("queue/logical_block_size"),
        source,
        "logical sector size is unavailable",
    );
    push_observation_issue(
        &mut issues,
        source,
        "logical sector size could not be read",
        &logical_sector_size_bytes,
    );
    let physical_sector_size_bytes = byte_observation(
        target.join("queue/physical_block_size"),
        source,
        "physical sector size is unavailable",
    );
    push_observation_issue(
        &mut issues,
        source,
        "physical sector size could not be read",
        &physical_sector_size_bytes,
    );

    if let Err(error) = fs::metadata(target)
        && !issues.iter().any(|issue| {
            issue.component == HardwareComponent::Storage
                && issue.detail == "block device disappeared during capture"
        })
    {
        issues.push(device_presence_issue(source, &error));
    }

    BuiltStorageDevice {
        device: StorageDevice {
            presentation: Observation::available(name.to_owned(), source, None),
            medium_technology,
            transport: transport(target, devices_root, source),
            nature: Observation::available(nature(name), source, None),
            removability,
            rotation: rotation_observation,
            supports_removable_media,
            capacity_bytes,
            reported_sector_size_bytes: Observation::unavailable(
                source,
                "no approved reported-sector source",
            ),
            logical_sector_size_bytes,
            physical_sector_size_bytes,
        },
        issues,
    }
}

fn device_presence_issue(source: DataSource, error: &io::Error) -> HardwareError {
    let (kind, detail) = match error.kind() {
        io::ErrorKind::NotFound => (
            HardwareErrorKind::Unavailable,
            "block device disappeared during capture",
        ),
        io::ErrorKind::PermissionDenied => (
            HardwareErrorKind::PermissionDenied,
            "block device presence could not be verified",
        ),
        _ => (
            HardwareErrorKind::ReadFailure,
            "block device presence could not be verified",
        ),
    };
    HardwareError::new(HardwareComponent::Storage, kind, source, detail)
}

fn push_attribute_issue(
    issues: &mut Vec<HardwareError>,
    source: DataSource,
    detail: &str,
    error: AttributeError,
) {
    let kind = match error {
        AttributeError::Unavailable => return,
        AttributeError::PermissionDenied => HardwareErrorKind::PermissionDenied,
        AttributeError::Invalid => HardwareErrorKind::InvalidData,
        AttributeError::ReadFailure => HardwareErrorKind::ReadFailure,
    };
    issues.push(HardwareError::new(
        HardwareComponent::Storage,
        kind,
        source,
        detail,
    ));
}

fn push_observation_issue<T>(
    issues: &mut Vec<HardwareError>,
    source: DataSource,
    detail: &str,
    observation: &Observation<T>,
) {
    let kind = match observation.status() {
        ObservationStatus::PermissionDenied => HardwareErrorKind::PermissionDenied,
        ObservationStatus::Error => HardwareErrorKind::ReadFailure,
        _ => return,
    };
    issues.push(HardwareError::new(
        HardwareComponent::Storage,
        kind,
        source,
        detail,
    ));
}

fn nature(name: &str) -> StorageNature {
    if name.starts_with("loop") {
        StorageNature::Loop
    } else if name.starts_with("zram") || name.starts_with("ram") {
        StorageNature::Ram
    } else if name.starts_with("dm-") {
        StorageNature::DeviceMapper
    } else if name.starts_with("md") {
        StorageNature::Raid
    } else {
        StorageNature::Physical
    }
}

fn transport(
    target: &Path,
    devices_root: &Path,
    source: DataSource,
) -> Observation<StorageTransport> {
    let subsystems = ancestor_subsystems(target, devices_root);
    let value = if subsystems.iter().any(|value| value == "nvme") {
        StorageTransport::Nvme
    } else if subsystems.iter().any(|value| value == "usb") {
        StorageTransport::Usb
    } else if subsystems.iter().any(|value| value == "ata") {
        StorageTransport::Ata
    } else if subsystems.iter().any(|value| value == "scsi") {
        StorageTransport::Scsi
    } else if subsystems.iter().any(|value| value == "mmc") {
        StorageTransport::Mmc
    } else if subsystems.iter().any(|value| value == "virtio") {
        StorageTransport::Virtio
    } else if subsystems.iter().any(|value| value == "pci") {
        StorageTransport::Pci
    } else {
        StorageTransport::Unknown
    };
    Observation::available(value, source, None)
}

fn physical_removability(
    target: &Path,
    devices_root: &Path,
    source: DataSource,
) -> Observation<Removability> {
    let ancestors = ancestors_with_subsystem(target, devices_root);
    let has_usb = ancestors.iter().any(|(_, subsystem)| subsystem == "usb");
    let relevant = if has_usb { "usb" } else { "pci" };
    for (candidate, _) in ancestors
        .iter()
        .filter(|(_, subsystem)| subsystem == relevant)
    {
        let value = match fs::read_to_string(candidate.join("removable")) {
            Ok(value) => value,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
                return Observation::permission_denied(
                    source,
                    "physical removability access was denied",
                );
            }
            Err(_) => {
                return Observation::error(source, "physical removability could not be read");
            }
        };
        return match value.trim() {
            "removable" => Observation::available(Removability::Removable, source, None),
            "fixed" => Observation::available(Removability::Fixed, source, None),
            "unknown" => Observation::available(Removability::Unknown, source, None),
            _ => Observation::error(source, "physical removability has an invalid value"),
        };
    }
    Observation::unavailable(source, "physical removability is not supported")
}

fn ancestor_subsystems(target: &Path, devices_root: &Path) -> Vec<String> {
    ancestors_with_subsystem(target, devices_root)
        .into_iter()
        .map(|(_, subsystem)| subsystem)
        .collect()
}

fn ancestors_with_subsystem(target: &Path, devices_root: &Path) -> Vec<(PathBuf, String)> {
    let mut result = Vec::new();
    let mut current = Some(target);
    while let Some(path) = current {
        if !path.starts_with(devices_root) {
            break;
        }
        if let Ok(link) = fs::read_link(path.join("subsystem"))
            && let Some(name) = link.file_name().and_then(|name| name.to_str())
        {
            result.push((path.to_path_buf(), name.to_owned()));
        }
        if path == devices_root {
            break;
        }
        current = path.parent();
    }
    result
}

fn byte_observation(path: PathBuf, source: DataSource, detail: &str) -> Observation<u64> {
    match read_number(path) {
        Ok(value) => Observation::available(value, source, Some(Unit::BytesPerSector)),
        Err(error) => attribute_failure(source, detail, error),
    }
}

#[derive(Clone, Copy)]
enum AttributeError {
    Unavailable,
    PermissionDenied,
    Invalid,
    ReadFailure,
}

fn read_number(path: PathBuf) -> Result<u64, AttributeError> {
    let value = read_attribute(path)?;
    value.trim().parse().map_err(|_| AttributeError::Invalid)
}

fn read_bool(path: PathBuf) -> Result<bool, AttributeError> {
    match read_attribute(path)?.trim() {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err(AttributeError::Invalid),
    }
}

fn read_attribute(path: PathBuf) -> Result<String, AttributeError> {
    fs::read_to_string(path).map_err(|error| match error.kind() {
        io::ErrorKind::NotFound => AttributeError::Unavailable,
        io::ErrorKind::PermissionDenied => AttributeError::PermissionDenied,
        _ => AttributeError::ReadFailure,
    })
}

fn attribute_failure<T>(
    source: DataSource,
    unavailable_detail: &str,
    error: AttributeError,
) -> Observation<T> {
    match error {
        AttributeError::Unavailable => Observation::unavailable(source, unavailable_detail),
        AttributeError::PermissionDenied => {
            Observation::permission_denied(source, "storage attribute access was denied")
        }
        AttributeError::Invalid => Observation::error(source, "storage attribute is invalid"),
        AttributeError::ReadFailure => {
            Observation::error(source, "storage attribute could not be read")
        }
    }
}

fn io_issue(source: DataSource, detail: &str, error: &io::Error) -> HardwareError {
    let kind = if error.kind() == io::ErrorKind::PermissionDenied {
        HardwareErrorKind::PermissionDenied
    } else {
        HardwareErrorKind::ReadFailure
    };
    HardwareError::new(HardwareComponent::Storage, kind, source, detail)
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use std::os::unix::fs::symlink;

    use super::*;

    fn removable_fixture(value: Option<&str>, include_usb: bool) -> tempfile::TempDir {
        let temp = tempfile::tempdir().expect("temporary fixture");
        let devices = temp.path().join("sys/devices");
        let pci = devices.join("pci0000");
        let usb = pci.join("usb1");
        let disk = if include_usb {
            usb.join("block/sda")
        } else {
            pci.join("block/sda")
        };
        fs::create_dir_all(&disk).expect("device fixture");
        fs::create_dir_all(temp.path().join("sys/bus/pci")).expect("PCI bus fixture");
        symlink("../../bus/pci", pci.join("subsystem")).expect("PCI subsystem link");
        fs::write(pci.join("removable"), "fixed\n").expect("PCI removable fixture");
        if include_usb {
            fs::create_dir_all(temp.path().join("sys/bus/usb")).expect("USB bus fixture");
            symlink("../../../bus/usb", usb.join("subsystem")).expect("USB subsystem link");
            if let Some(value) = value {
                fs::write(usb.join("removable"), format!("{value}\n"))
                    .expect("USB removable fixture");
            }
        } else if let Some(value) = value {
            fs::write(pci.join("removable"), format!("{value}\n")).expect("PCI removable fixture");
        }
        temp
    }

    #[test]
    fn virtual_device_natures_are_explicit() {
        assert_eq!(nature("loop0"), StorageNature::Loop);
        assert_eq!(nature("zram0"), StorageNature::Ram);
        assert_eq!(nature("dm-0"), StorageNature::DeviceMapper);
        assert_eq!(nature("md0"), StorageNature::Raid);
        assert_eq!(nature("sda"), StorageNature::Physical);
    }

    #[test]
    fn physical_removability_preserves_all_defined_values() {
        for (raw, expected) in [
            ("removable", Removability::Removable),
            ("fixed", Removability::Fixed),
            ("unknown", Removability::Unknown),
        ] {
            let fixture = removable_fixture(Some(raw), true);
            let devices = fixture.path().join("sys/devices");
            let observation = physical_removability(
                &devices.join("pci0000/usb1/block/sda"),
                &devices,
                DataSource::LinuxSysfsBlockLegacy,
            );
            assert_eq!(observation.value(), Some(&expected));
        }
    }

    #[test]
    fn missing_usb_attribute_does_not_fall_back_to_pci() {
        let fixture = removable_fixture(None, true);
        let devices = fixture.path().join("sys/devices");
        let observation = physical_removability(
            &devices.join("pci0000/usb1/block/sda"),
            &devices,
            DataSource::LinuxSysfsBlockLegacy,
        );
        assert_eq!(
            observation.status(),
            adaptive_domain::ObservationStatus::Unavailable
        );
        assert_eq!(observation.value(), None);
    }

    #[test]
    fn pci_without_attribute_is_unavailable() {
        let fixture = removable_fixture(None, false);
        fs::remove_file(fixture.path().join("sys/devices/pci0000/removable"))
            .expect("remove PCI removable fixture");
        let devices = fixture.path().join("sys/devices");
        let observation = physical_removability(
            &devices.join("pci0000/block/sda"),
            &devices,
            DataSource::LinuxSysfsBlockLegacy,
        );
        assert_eq!(
            observation.status(),
            adaptive_domain::ObservationStatus::Unavailable
        );
    }

    #[test]
    fn vanished_device_produces_a_sanitized_incident() {
        let fixture = tempfile::tempdir().expect("temporary fixture");
        let devices = fixture.path().join("sys/devices");
        fs::create_dir_all(&devices).expect("devices root");
        let built = build_storage_device(
            "sda",
            &devices.join("vanished/block/sda"),
            &devices,
            DataSource::LinuxSysfsBlockLegacy,
        );

        assert!(built.issues.iter().any(|issue| {
            issue.kind == HardwareErrorKind::Unavailable
                && issue.detail == "block device disappeared during capture"
        }));
        assert!(built.issues.iter().all(|issue| {
            !issue
                .detail
                .contains(fixture.path().to_string_lossy().as_ref())
        }));
    }

    #[test]
    fn permission_denied_attribute_produces_status_and_incident() {
        let observation = attribute_failure::<u64>(
            DataSource::LinuxSysfsBlockLegacy,
            "attribute unavailable",
            AttributeError::PermissionDenied,
        );
        let mut issues = Vec::new();
        push_attribute_issue(
            &mut issues,
            DataSource::LinuxSysfsBlockLegacy,
            "attribute access denied",
            AttributeError::PermissionDenied,
        );
        assert_eq!(observation.status(), ObservationStatus::PermissionDenied);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].kind, HardwareErrorKind::PermissionDenied);
    }
}
