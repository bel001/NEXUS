use std::collections::BTreeSet;

use adaptive_domain::{
    Architecture, BatteryDirection, BatteryLevel, Rotation, StorageMediumTechnology,
    StorageTransport,
};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CpuRecord {
    pub manufacturer: Option<String>,
    pub name: Option<String>,
    pub cores: Option<u32>,
    pub logical_processors: Option<u32>,
    pub architecture: Option<u16>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AggregatedCpu {
    pub manufacturer: Option<String>,
    pub name: Option<String>,
    pub cores: Option<u32>,
    pub logical_processors: Option<u32>,
    pub architecture: Option<Architecture>,
    pub architecture_issue: Option<CpuArchitectureIssue>,
    pub duplicate_rows_removed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CpuArchitectureIssue {
    UnknownCode,
    ConflictingValues,
}

#[cfg(any(target_os = "windows", test))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PrimaryStorageDecision {
    UsePrimary,
    QueryFallback,
}

#[cfg(any(target_os = "windows", test))]
pub(crate) fn primary_storage_decision(row_count: Option<usize>) -> PrimaryStorageDecision {
    match row_count {
        Some(count) if count > 0 => PrimaryStorageDecision::UsePrimary,
        Some(_) | None => PrimaryStorageDecision::QueryFallback,
    }
}

pub fn processor_architecture(code: u16) -> Result<Architecture, u16> {
    match code {
        0 => Ok(Architecture::X86),
        1 => Ok(Architecture::Mips),
        2 => Ok(Architecture::Alpha),
        3 => Ok(Architecture::PowerPc),
        5 => Ok(Architecture::Arm),
        6 => Ok(Architecture::Itanium),
        9 => Ok(Architecture::X86_64),
        12 => Ok(Architecture::Arm64),
        _ => Err(code),
    }
}

pub fn aggregate_cpu(records: Vec<CpuRecord>) -> Option<AggregatedCpu> {
    if records.is_empty() {
        return None;
    }
    let original_len = records.len();
    let unique: BTreeSet<CpuRecord> = records.into_iter().collect();
    let duplicate_rows_removed = unique.len() != original_len;
    let manufacturer = unique.iter().find_map(|record| record.manufacturer.clone());
    let name = unique.iter().find_map(|record| record.name.clone());
    let cores = checked_sum(unique.iter().map(|record| record.cores));
    let logical_processors = checked_sum(unique.iter().map(|record| record.logical_processors));
    let architecture_codes: Vec<u16> = unique
        .iter()
        .filter_map(|record| record.architecture)
        .collect();
    let mut unknown_architecture = false;
    let architectures: BTreeSet<Architecture> = architecture_codes
        .iter()
        .filter_map(|code| match processor_architecture(*code) {
            Ok(value) => Some(value),
            Err(_) => {
                unknown_architecture = true;
                None
            }
        })
        .collect();
    let (architecture, architecture_issue) = if architecture_codes.is_empty() {
        (None, None)
    } else if unknown_architecture {
        (
            Some(Architecture::Unknown),
            Some(CpuArchitectureIssue::UnknownCode),
        )
    } else if architectures.len() == 1 {
        (architectures.first().copied(), None)
    } else {
        (
            Some(Architecture::Unknown),
            Some(CpuArchitectureIssue::ConflictingValues),
        )
    };
    Some(AggregatedCpu {
        manufacturer,
        name,
        cores,
        logical_processors,
        architecture,
        architecture_issue,
        duplicate_rows_removed,
    })
}

fn checked_sum(values: impl Iterator<Item = Option<u32>>) -> Option<u32> {
    let values: Option<Vec<u32>> = values.collect();
    values?.into_iter().try_fold(0_u32, u32::checked_add)
}

pub fn spindle_speed(value: u32) -> Rotation {
    match value {
        0 => Rotation::NonRotational,
        u32::MAX => Rotation::Unknown,
        _ => Rotation::Rotational,
    }
}

pub fn physical_disk_media_type(value: u16) -> Result<StorageMediumTechnology, u16> {
    match value {
        3 => Ok(StorageMediumTechnology::Magnetic),
        4 => Ok(StorageMediumTechnology::SolidState),
        5 => Ok(StorageMediumTechnology::StorageClassMemory),
        _ => Err(value),
    }
}

/// Tabla específica de `MSFT_PhysicalDisk.BusType`; no es STORAGE_BUS_TYPE.
pub fn physical_disk_bus_type(value: u16) -> Result<StorageTransport, u16> {
    match value {
        1 => Ok(StorageTransport::Scsi),
        3 => Ok(StorageTransport::Ata),
        7 => Ok(StorageTransport::Usb),
        10 => Ok(StorageTransport::Sas),
        11 => Ok(StorageTransport::Sata),
        13 => Ok(StorageTransport::Mmc),
        17 => Ok(StorageTransport::Nvme),
        _ => Err(value),
    }
}

pub fn battery_status(value: u16) -> Result<(BatteryDirection, BatteryLevel), u16> {
    match value {
        1 | 2 | 10 => Ok((BatteryDirection::Unknown, BatteryLevel::Unknown)),
        3 => Ok((BatteryDirection::NotCharging, BatteryLevel::Full)),
        4 => Ok((BatteryDirection::Unknown, BatteryLevel::Low)),
        5 => Ok((BatteryDirection::Unknown, BatteryLevel::Critical)),
        6 => Ok((BatteryDirection::Charging, BatteryLevel::Unknown)),
        7 => Ok((BatteryDirection::Charging, BatteryLevel::High)),
        8 => Ok((BatteryDirection::Charging, BatteryLevel::Low)),
        9 => Ok((BatteryDirection::Charging, BatteryLevel::Critical)),
        11 => Ok((BatteryDirection::Unknown, BatteryLevel::Partial)),
        _ => Err(value),
    }
}

pub fn storage_evidence_is_contradictory(
    medium: StorageMediumTechnology,
    rotation: Rotation,
) -> bool {
    matches!(
        (medium, rotation),
        (StorageMediumTechnology::Magnetic, Rotation::NonRotational)
            | (StorageMediumTechnology::SolidState, Rotation::Rotational)
            | (
                StorageMediumTechnology::StorageClassMemory,
                Rotation::Rotational
            )
    )
}

pub fn visible_memory_bytes(kibibytes: u64) -> Option<u64> {
    kibibytes.checked_mul(1024)
}

pub fn supports_removable_media(capabilities: Option<&[u16]>) -> Option<bool> {
    capabilities.map(|values| values.contains(&7))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn architecture_table_keeps_unknown_codes() {
        assert_eq!(processor_architecture(9), Ok(Architecture::X86_64));
        assert_eq!(processor_architecture(99), Err(99));
    }

    #[test]
    fn duplicate_cpu_rows_are_not_summed_twice() {
        let row = CpuRecord {
            manufacturer: Some("Example".into()),
            name: Some("CPU".into()),
            cores: Some(4),
            logical_processors: Some(8),
            architecture: Some(9),
        };
        let result = aggregate_cpu(vec![row.clone(), row]).expect("records exist");
        assert_eq!(result.cores, Some(4));
        assert_eq!(result.logical_processors, Some(8));
        assert!(result.duplicate_rows_removed);
    }

    #[test]
    fn spindle_speed_does_not_infer_media_type() {
        assert_eq!(spindle_speed(0), Rotation::NonRotational);
        assert_eq!(spindle_speed(u32::MAX), Rotation::Unknown);
        assert_eq!(physical_disk_media_type(0), Err(0));
    }

    #[test]
    fn low_battery_does_not_imply_discharging() {
        assert_eq!(
            battery_status(4),
            Ok((BatteryDirection::Unknown, BatteryLevel::Low))
        );
        assert_eq!(
            battery_status(5),
            Ok((BatteryDirection::Unknown, BatteryLevel::Critical))
        );
    }

    #[test]
    fn reserved_bus_values_remain_unknown() {
        assert_eq!(physical_disk_bus_type(18), Err(18));
        assert_eq!(physical_disk_bus_type(20), Err(20));
    }

    #[test]
    fn memory_conversion_is_checked() {
        assert_eq!(visible_memory_bytes(1024), Some(1_048_576));
        assert_eq!(visible_memory_bytes(u64::MAX), None);
    }

    #[test]
    fn fallback_removable_capability_is_explicit() {
        assert_eq!(supports_removable_media(Some(&[3, 7])), Some(true));
        assert_eq!(supports_removable_media(Some(&[3])), Some(false));
        assert_eq!(supports_removable_media(None), None);
    }

    #[test]
    fn absent_cpu_architecture_stays_absent() {
        let result = aggregate_cpu(vec![CpuRecord {
            manufacturer: Some("Example".into()),
            name: Some("CPU".into()),
            cores: Some(4),
            logical_processors: Some(8),
            architecture: None,
        }])
        .expect("record exists");
        assert_eq!(result.architecture, None);
        assert_eq!(result.architecture_issue, None);
    }

    #[test]
    fn unknown_cpu_architecture_retains_a_diagnostic() {
        let result = aggregate_cpu(vec![CpuRecord {
            manufacturer: None,
            name: None,
            cores: Some(4),
            logical_processors: Some(8),
            architecture: Some(99),
        }])
        .expect("record exists");
        assert_eq!(result.architecture, Some(Architecture::Unknown));
        assert_eq!(
            result.architecture_issue,
            Some(CpuArchitectureIssue::UnknownCode)
        );
    }

    #[test]
    fn storage_class_memory_and_contradictions_are_explicit() {
        assert_eq!(
            physical_disk_media_type(5),
            Ok(StorageMediumTechnology::StorageClassMemory)
        );
        assert!(storage_evidence_is_contradictory(
            StorageMediumTechnology::Magnetic,
            Rotation::NonRotational
        ));
        assert!(storage_evidence_is_contradictory(
            StorageMediumTechnology::SolidState,
            Rotation::Rotational
        ));
        assert!(!storage_evidence_is_contradictory(
            StorageMediumTechnology::SolidState,
            Rotation::NonRotational
        ));
        assert!(!storage_evidence_is_contradictory(
            StorageMediumTechnology::Magnetic,
            Rotation::Unknown
        ));
    }

    #[test]
    fn unknown_battery_status_is_distinct_from_known_unknown() {
        assert_eq!(
            battery_status(2),
            Ok((BatteryDirection::Unknown, BatteryLevel::Unknown))
        );
        assert_eq!(battery_status(99), Err(99));
    }

    #[test]
    fn storage_policy_uses_primary_only_for_nonempty_success() {
        assert_eq!(
            primary_storage_decision(Some(2)),
            PrimaryStorageDecision::UsePrimary
        );
        assert_eq!(
            primary_storage_decision(Some(0)),
            PrimaryStorageDecision::QueryFallback
        );
        assert_eq!(
            primary_storage_decision(None),
            PrimaryStorageDecision::QueryFallback
        );
    }
}
