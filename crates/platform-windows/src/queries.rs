pub(crate) const OS_QUERY: &str =
    "SELECT Caption, Version, OSArchitecture, TotalVisibleMemorySize FROM Win32_OperatingSystem";
pub(crate) const CPU_QUERY: &str = "SELECT Manufacturer, Name, NumberOfCores, NumberOfLogicalProcessors, Architecture FROM Win32_Processor";
pub(crate) const GPU_QUERY: &str = "SELECT Name, AdapterCompatibility FROM Win32_VideoController";
pub(crate) const COMPUTER_QUERY: &str = "SELECT Manufacturer, Model FROM Win32_ComputerSystem";
pub(crate) const BATTERY_QUERY: &str =
    "SELECT EstimatedChargeRemaining, BatteryStatus FROM Win32_Battery";
pub(crate) const PHYSICAL_DISK_QUERY: &str = "SELECT FriendlyName, MediaType, BusType, Size, LogicalSectorSize, PhysicalSectorSize, SpindleSpeed FROM MSFT_PhysicalDisk";
pub(crate) const DISK_DRIVE_QUERY: &str = "SELECT Model, Manufacturer, MediaType, Size, BytesPerSector, Capabilities FROM Win32_DiskDrive";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_query_is_a_fixed_projection() {
        for query in [
            OS_QUERY,
            CPU_QUERY,
            GPU_QUERY,
            COMPUTER_QUERY,
            BATTERY_QUERY,
            PHYSICAL_DISK_QUERY,
            DISK_DRIVE_QUERY,
        ] {
            assert!(query.starts_with("SELECT "));
            assert!(!query.contains("SELECT *"));
            assert!(!query.contains(';'));
        }
    }
}
