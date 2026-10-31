#![cfg(target_os = "linux")]

use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;

use adaptive_domain::{
    BatteryDirection, BatteryLevel, CapabilityKind, DataSource, HardwareComponent,
    HardwareErrorKind, ObservationStatus, Removability, Rotation, StorageNature, Unit,
};
use adaptive_hardware_discovery::HardwareProvider;
use adaptive_platform_linux::LinuxHardwareProvider;

fn write(path: &Path, value: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("fixture parent");
    }
    fs::write(path, value).expect("fixture file");
}

fn base_fixture(root: &Path) {
    write(
        &root.join("etc/os-release"),
        include_str!("fixtures/os-release"),
    );
    write(
        &root.join("proc/cpuinfo"),
        include_str!("fixtures/cpuinfo.txt"),
    );
    write(
        &root.join("proc/meminfo"),
        include_str!("fixtures/meminfo.txt"),
    );
    write(&root.join("sys/devices/system/cpu/online"), "0-1\n");
    for cpu in [0, 1] {
        let topology = root.join(format!("sys/devices/system/cpu/cpu{cpu}/topology"));
        write(&topology.join("core_cpus_list"), &format!("{cpu}\n"));
        write(&topology.join("physical_package_id"), "0\n");
    }
    write(
        &root.join("sys/class/dmi/id/sys_vendor"),
        "Fixture Vendor\n",
    );
    write(
        &root.join("sys/class/dmi/id/product_name"),
        "Fixture Model\n",
    );
    fs::create_dir_all(root.join("sys/class/drm")).expect("DRM fixture");
    fs::create_dir_all(root.join("sys/class/power_supply")).expect("battery fixture");
}

fn legacy_disk_fixture(root: &Path) {
    let disk = root.join("sys/devices/pci0000/block/sda");
    write(&disk.join("size"), "2048\n");
    write(&disk.join("queue/rotational"), "0\n");
    write(&disk.join("queue/logical_block_size"), "512\n");
    write(&disk.join("queue/physical_block_size"), "4096\n");
    write(&disk.join("removable"), "0\n");
    write(&root.join("sys/devices/pci0000/removable"), "fixed\n");
    fs::create_dir_all(root.join("sys/bus/pci")).expect("PCI bus fixture");
    symlink("../../bus/pci", root.join("sys/devices/pci0000/subsystem"))
        .expect("PCI subsystem link");

    fs::create_dir_all(root.join("sys/class/block")).expect("class block fixture");
    fs::create_dir_all(root.join("sys/block")).expect("block fixture");
    symlink(
        "../../devices/pci0000/block/sda",
        root.join("sys/class/block/sda"),
    )
    .expect("class block link");
    symlink("../devices/pci0000/block/sda", root.join("sys/block/sda")).expect("legacy block link");

    let partition = disk.join("sda1");
    write(&partition.join("partition"), "1\n");
    symlink(
        "../../devices/pci0000/block/sda/sda1",
        root.join("sys/class/block/sda1"),
    )
    .expect("partition link");
}

#[test]
fn fixture_inventory_normalizes_and_deduplicates() {
    let temp = tempfile::tempdir().expect("temporary fixture");
    base_fixture(temp.path());
    legacy_disk_fixture(temp.path());

    let inventory = LinuxHardwareProvider::with_root(temp.path())
        .collect()
        .expect("provider remains usable");

    let memory = inventory.memory.value().expect("memory component");
    assert_eq!(
        memory.total_visible_bytes.value(),
        Some(&(8_388_608_u64 * 1024))
    );
    assert_eq!(memory.total_visible_bytes.unit(), Some(Unit::Bytes));

    let cpu = inventory.cpu.value().expect("CPU component");
    assert_eq!(cpu.logical_processor_count.value(), Some(&2));
    assert_eq!(cpu.logical_processor_count.source(), DataSource::LinuxSysfs);
    assert_eq!(cpu.online_physical_core_count.value(), Some(&2));
    assert_eq!(cpu.package_count.value(), Some(&1));

    let disks = inventory.storage.value().expect("storage collection");
    assert_eq!(disks.len(), 1, "legacy aliases and partitions are removed");
    let disk = &disks[0];
    assert_eq!(disk.capacity_bytes.value(), Some(&1_048_576));
    assert_eq!(disk.rotation.value(), Some(&Rotation::NonRotational));
    assert_eq!(disk.nature.value(), Some(&StorageNature::Physical));
    assert_eq!(disk.removability.value(), Some(&Removability::Fixed));

    assert_eq!(inventory.batteries.value().map(Vec::len), Some(0));
    assert!(
        inventory.captured_at.is_none(),
        "provider does not own the clock"
    );
}

#[test]
fn modern_layout_never_falls_back_to_legacy() {
    let temp = tempfile::tempdir().expect("temporary fixture");
    base_fixture(temp.path());
    legacy_disk_fixture(temp.path());
    fs::create_dir_all(temp.path().join("sys/subsystem")).expect("modern root");

    let inventory = LinuxHardwareProvider::with_root(temp.path())
        .collect()
        .expect("provider remains usable");
    assert_eq!(inventory.storage.status(), ObservationStatus::Unavailable);
}

#[test]
fn modern_layout_uses_only_subsystem_devices() {
    let temp = tempfile::tempdir().expect("temporary fixture");
    base_fixture(temp.path());
    legacy_disk_fixture(temp.path());
    fs::create_dir_all(temp.path().join("sys/subsystem/block/devices")).expect("modern block root");
    symlink(
        "../../../devices/pci0000/block/sda",
        temp.path().join("sys/subsystem/block/devices/sda"),
    )
    .expect("modern block link");

    let inventory = LinuxHardwareProvider::with_root(temp.path())
        .collect()
        .expect("provider remains usable");
    assert_eq!(inventory.storage.status(), ObservationStatus::Available);
    assert_eq!(inventory.storage.value().map(Vec::len), Some(1));
}

#[test]
fn broken_legacy_link_yields_a_partial_collection() {
    let temp = tempfile::tempdir().expect("temporary fixture");
    base_fixture(temp.path());
    legacy_disk_fixture(temp.path());
    symlink(
        "../../devices/missing/block/sdb",
        temp.path().join("sys/class/block/sdb"),
    )
    .expect("broken block link");

    let inventory = LinuxHardwareProvider::with_root(temp.path())
        .collect()
        .expect("provider remains usable");
    assert_eq!(inventory.storage.status(), ObservationStatus::Partial);
    assert_eq!(inventory.storage.value().map(Vec::len), Some(1));
    assert!(
        inventory
            .issues
            .iter()
            .any(|issue| issue.component == adaptive_domain::HardwareComponent::Storage)
    );
}

#[test]
fn incomplete_topology_is_not_invented() {
    let temp = tempfile::tempdir().expect("temporary fixture");
    base_fixture(temp.path());
    fs::remove_file(
        temp.path()
            .join("sys/devices/system/cpu/cpu1/topology/core_cpus_list"),
    )
    .expect("remove fixture field");
    fs::create_dir_all(temp.path().join("sys/class/block")).expect("block fixture");

    let inventory = LinuxHardwareProvider::with_root(temp.path())
        .collect()
        .expect("provider remains usable");
    let cpu = inventory.cpu.value().expect("CPU component");
    assert_eq!(
        cpu.online_physical_core_count.status(),
        ObservationStatus::Unavailable
    );
}

#[test]
fn procfs_logical_cpu_fallback_preserves_its_source() {
    let temp = tempfile::tempdir().expect("temporary fixture");
    base_fixture(temp.path());
    fs::remove_file(temp.path().join("sys/devices/system/cpu/online"))
        .expect("remove sysfs online set");
    fs::create_dir_all(temp.path().join("sys/class/block")).expect("block fixture");

    let inventory = LinuxHardwareProvider::with_root(temp.path())
        .collect()
        .expect("provider remains usable");
    let cpu = inventory.cpu.value().expect("CPU component");
    assert_eq!(
        cpu.logical_processor_count.source(),
        DataSource::LinuxProcfs
    );
}

#[test]
fn manager_capabilities_are_not_false_without_evidence() {
    let temp = tempfile::tempdir().expect("temporary fixture");
    base_fixture(temp.path());
    fs::create_dir_all(temp.path().join("sys/class/block")).expect("block fixture");

    let inventory = LinuxHardwareProvider::with_root(temp.path())
        .collect()
        .expect("provider remains usable");
    let capabilities = inventory.capabilities.value().expect("capabilities");
    let power_profiles = capabilities
        .iter()
        .find(|capability| capability.kind == CapabilityKind::PowerProfilesDaemon)
        .expect("power-profiles-daemon capability");
    match power_profiles.available.status() {
        ObservationStatus::Available => {
            assert!(power_profiles.available.value().is_some());
            assert_eq!(power_profiles.available.source(), DataSource::LinuxDbus);
            assert!(power_profiles.available.detail().is_some());
        }
        status => {
            assert!(matches!(
                status,
                ObservationStatus::Unavailable
                    | ObservationStatus::PermissionDenied
                    | ObservationStatus::Error
            ));
            assert_eq!(power_profiles.available.value(), None);
        }
    }

    for kind in [CapabilityKind::Tlp, CapabilityKind::AutoCpuFreq] {
        let capability = capabilities
            .iter()
            .find(|capability| capability.kind == kind)
            .expect("manager capability");
        assert_eq!(
            capability.available.status(),
            ObservationStatus::Unavailable
        );
        assert_eq!(capability.available.value(), None);
    }
}

#[test]
fn contradictory_topology_is_unavailable_and_reported() {
    let temp = tempfile::tempdir().expect("temporary fixture");
    base_fixture(temp.path());
    write(
        &temp
            .path()
            .join("sys/devices/system/cpu/cpu0/topology/core_cpus_list"),
        "0-1\n",
    );
    write(
        &temp
            .path()
            .join("sys/devices/system/cpu/cpu1/topology/core_cpus_list"),
        "1\n",
    );
    fs::create_dir_all(temp.path().join("sys/class/block")).expect("block fixture");

    let inventory = LinuxHardwareProvider::with_root(temp.path())
        .collect()
        .expect("provider remains usable");
    let cpu = inventory.cpu.value().expect("CPU component");
    assert_eq!(
        cpu.online_physical_core_count.status(),
        ObservationStatus::Unavailable
    );
    assert!(inventory.issues.iter().any(|issue| {
        issue.component == HardwareComponent::Cpu && issue.kind == HardwareErrorKind::InvalidData
    }));
}

#[test]
fn legacy_views_are_unioned_without_merging_distinct_devices() {
    let temp = tempfile::tempdir().expect("temporary fixture");
    base_fixture(temp.path());
    legacy_disk_fixture(temp.path());

    let loop_device = temp.path().join("sys/devices/virtual/block/loop0");
    write(&loop_device.join("size"), "64\n");
    write(&loop_device.join("queue/rotational"), "0\n");
    write(&loop_device.join("queue/logical_block_size"), "512\n");
    write(&loop_device.join("queue/physical_block_size"), "512\n");
    write(&loop_device.join("removable"), "0\n");
    fs::create_dir_all(temp.path().join("sys/bus/block/devices")).expect("bus block fixture");
    symlink(
        "../../../devices/virtual/block/loop0",
        temp.path().join("sys/bus/block/devices/loop0"),
    )
    .expect("bus block link");

    let inventory = LinuxHardwareProvider::with_root(temp.path())
        .collect()
        .expect("provider remains usable");
    let disks = inventory.storage.value().expect("storage collection");
    assert_eq!(disks.len(), 2);
    assert!(
        disks
            .iter()
            .any(|disk| disk.nature.value() == Some(&StorageNature::Loop))
    );
}

#[test]
fn legacy_device_link_can_recover_a_classification_entry() {
    let temp = tempfile::tempdir().expect("temporary fixture");
    base_fixture(temp.path());
    let disk = temp.path().join("sys/devices/recovered/block/sdc");
    write(&disk.join("size"), "128\n");
    write(&disk.join("queue/rotational"), "1\n");
    write(&disk.join("queue/logical_block_size"), "512\n");
    write(&disk.join("queue/physical_block_size"), "4096\n");
    write(&disk.join("removable"), "0\n");

    let legacy_entry = temp.path().join("legacy/sdc");
    fs::create_dir_all(&legacy_entry).expect("legacy entry");
    symlink(
        "../../sys/devices/recovered/block/sdc",
        legacy_entry.join("device"),
    )
    .expect("conditional device link");
    fs::create_dir_all(temp.path().join("sys/class/block")).expect("class fixture");
    symlink(
        "../../../legacy/sdc",
        temp.path().join("sys/class/block/sdc"),
    )
    .expect("external classification link");

    let inventory = LinuxHardwareProvider::with_root(temp.path())
        .collect()
        .expect("provider remains usable");
    assert_eq!(inventory.storage.value().map(Vec::len), Some(1));
    assert_eq!(inventory.storage.status(), ObservationStatus::Available);
}

#[test]
fn external_legacy_target_is_reported_without_exposing_its_path() {
    let temp = tempfile::tempdir().expect("temporary fixture");
    base_fixture(temp.path());
    fs::create_dir_all(temp.path().join("external/sdx")).expect("external target");
    fs::create_dir_all(temp.path().join("sys/class/block")).expect("class fixture");
    symlink(
        "../../../external/sdx",
        temp.path().join("sys/class/block/sdx"),
    )
    .expect("external classification link");

    let inventory = LinuxHardwareProvider::with_root(temp.path())
        .collect()
        .expect("provider remains usable");
    assert_eq!(inventory.storage.status(), ObservationStatus::Partial);
    assert_eq!(inventory.storage.value().map(Vec::len), Some(0));
    assert!(inventory.issues.iter().any(|issue| {
        issue.component == HardwareComponent::Storage
            && !issue
                .detail
                .contains(temp.path().to_string_lossy().as_ref())
    }));
}

#[test]
fn symlink_loop_is_reported_as_a_partial_entry_failure() {
    let temp = tempfile::tempdir().expect("temporary fixture");
    base_fixture(temp.path());
    fs::create_dir_all(temp.path().join("sys/class/block")).expect("class fixture");
    symlink("looped", temp.path().join("sys/class/block/looped"))
        .expect("self-referential classification link");

    let inventory = LinuxHardwareProvider::with_root(temp.path())
        .collect()
        .expect("provider remains usable");
    assert_eq!(inventory.storage.status(), ObservationStatus::Partial);
    assert!(
        inventory
            .issues
            .iter()
            .any(|issue| issue.component == HardwareComponent::Storage)
    );
}

#[test]
fn populated_battery_is_normalized() {
    let temp = tempfile::tempdir().expect("temporary fixture");
    base_fixture(temp.path());
    fs::create_dir_all(temp.path().join("sys/class/block")).expect("block fixture");
    let battery = temp.path().join("sys/class/power_supply/BAT0");
    write(&battery.join("type"), "Battery\n");
    write(&battery.join("capacity"), "73\n");
    write(&battery.join("status"), "Discharging\n");
    write(&battery.join("capacity_level"), "Normal\n");

    let inventory = LinuxHardwareProvider::with_root(temp.path())
        .collect()
        .expect("provider remains usable");
    let batteries = inventory.batteries.value().expect("battery collection");
    assert_eq!(batteries.len(), 1);
    assert_eq!(batteries[0].percentage.value(), Some(&73));
    assert_eq!(
        batteries[0].direction.value(),
        Some(&BatteryDirection::Discharging)
    );
    assert_eq!(batteries[0].level.value(), Some(&BatteryLevel::High));
}

#[test]
fn incomplete_battery_keeps_partial_data() {
    let temp = tempfile::tempdir().expect("temporary fixture");
    base_fixture(temp.path());
    fs::create_dir_all(temp.path().join("sys/class/block")).expect("block fixture");
    let battery = temp.path().join("sys/class/power_supply/BAT0");
    write(&battery.join("type"), "Battery\n");
    write(&battery.join("capacity"), "40\n");
    write(&battery.join("status"), "Charging\n");

    let inventory = LinuxHardwareProvider::with_root(temp.path())
        .collect()
        .expect("provider remains usable");
    let batteries = inventory.batteries.value().expect("battery collection");
    assert_eq!(inventory.batteries.status(), ObservationStatus::Partial);
    assert_eq!(batteries[0].percentage.value(), Some(&40));
    assert_eq!(batteries[0].level.status(), ObservationStatus::Unavailable);
}
