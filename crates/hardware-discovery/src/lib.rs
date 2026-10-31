#![forbid(unsafe_code)]

//! Contrato de aplicación y orquestación de Hardware Discovery.

use std::time::SystemTime;

use adaptive_domain::{
    DataSource, HARDWARE_INVENTORY_SCHEMA_VERSION, HardwareComponent, HardwareError,
    HardwareErrorKind, HardwareInventory,
};

/// Adaptador de plataforma. No exige `Send` ni `Sync`; WMI puede permanecer
/// confinado al hilo que crea su conexión.
pub trait HardwareProvider {
    fn collect(&self) -> Result<HardwareInventory, HardwareError>;
}

/// Ejecuta una captura usando el reloj civil del proceso.
pub fn capture(provider: &impl HardwareProvider) -> HardwareInventory {
    capture_at(provider, Some(SystemTime::now()))
}

/// Variante determinista para pruebas y entornos sin reloj civil fiable.
pub fn capture_at(
    provider: &impl HardwareProvider,
    captured_at: Option<SystemTime>,
) -> HardwareInventory {
    let mut inventory = match provider.collect() {
        Ok(inventory) => inventory,
        Err(error) => {
            let mut partial = HardwareInventory::empty();
            partial.issues.push(HardwareError::new(
                HardwareComponent::Provider,
                HardwareErrorKind::ProviderFailure,
                DataSource::Orchestrator,
                error.to_string(),
            ));
            partial
        }
    };

    inventory.schema_version = HARDWARE_INVENTORY_SCHEMA_VERSION;
    inventory.captured_at = None;
    if let Some(timestamp) = captured_at {
        inventory.set_captured_at_from_system_time(timestamp);
    }
    inventory
}

#[cfg(test)]
mod tests {
    use std::time::UNIX_EPOCH;

    use adaptive_domain::{HARDWARE_INVENTORY_SCHEMA_VERSION, ObservationStatus};

    use super::*;

    struct CompleteProvider;

    impl HardwareProvider for CompleteProvider {
        fn collect(&self) -> Result<HardwareInventory, HardwareError> {
            Ok(HardwareInventory::empty())
        }
    }

    struct FailingProvider;

    impl HardwareProvider for FailingProvider {
        fn collect(&self) -> Result<HardwareInventory, HardwareError> {
            Err(HardwareError::new(
                HardwareComponent::Provider,
                HardwareErrorKind::ProviderFailure,
                DataSource::Runtime,
                "provider unavailable",
            ))
        }
    }

    #[test]
    fn orchestrator_applies_one_common_timestamp() {
        let inventory = capture_at(&CompleteProvider, Some(UNIX_EPOCH));
        assert!(inventory.captured_at.is_some());
        assert_eq!(inventory.schema_version, HARDWARE_INVENTORY_SCHEMA_VERSION);
    }

    #[test]
    fn clock_can_be_unavailable() {
        let inventory = capture_at(&CompleteProvider, None);
        assert!(inventory.captured_at.is_none());
    }

    #[test]
    fn fatal_provider_error_becomes_partial_inventory() {
        let inventory = capture_at(&FailingProvider, None);
        assert_eq!(inventory.issues.len(), 1);
        assert_eq!(
            inventory.operating_system.status(),
            ObservationStatus::Unavailable
        );
    }
}
