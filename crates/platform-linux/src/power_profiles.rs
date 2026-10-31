use adaptive_domain::{
    CapabilityInfo, CapabilityKind, DataSource, HardwareComponent, HardwareError,
    HardwareErrorKind, Observation,
};

#[cfg(target_os = "linux")]
const SERVICE: &str = "org.freedesktop.UPower.PowerProfiles";
#[cfg(target_os = "linux")]
const PATH: &str = "/org/freedesktop/UPower/PowerProfiles";
#[cfg(target_os = "linux")]
const INTERFACE: &str = "org.freedesktop.UPower.PowerProfiles";
#[cfg(any(target_os = "linux", test))]
const READ_PROPERTIES: [&str; 3] = ["Profiles", "Version", "ActiveProfile"];
#[cfg(target_os = "linux")]
const METHOD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

#[derive(Clone, Debug, Eq, PartialEq)]
struct PowerProfilesEvidence {
    profile_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
enum ProbeFailure {
    Connection,
    PermissionDenied,
    Timeout,
    MissingProperty(&'static str),
    InvalidProperty(&'static str),
    ReadFailure,
}

trait PowerProfilesProbe {
    fn probe(&self) -> Result<Option<PowerProfilesEvidence>, ProbeFailure>;
}

struct SystemPowerProfilesProbe;

pub(super) fn detect() -> (CapabilityInfo, Option<HardwareError>) {
    capability_from_probe(&SystemPowerProfilesProbe)
}

fn capability_from_probe(
    probe: &impl PowerProfilesProbe,
) -> (CapabilityInfo, Option<HardwareError>) {
    let (available, issue) = match probe.probe() {
        Ok(Some(evidence)) => (
            Observation::available_with_detail(
                true,
                DataSource::LinuxDbus,
                None,
                format!(
                    "D-Bus service owns its name; Profiles contains {} entries; Version and ActiveProfile are readable",
                    evidence.profile_count
                ),
            ),
            None,
        ),
        Ok(None) => (
            Observation::available_with_detail(
                false,
                DataSource::LinuxDbus,
                None,
                "the system bus reports that the D-Bus service has no owner",
            ),
            None,
        ),
        Err(failure) => {
            let (observation, kind, detail) = match failure {
                ProbeFailure::Connection => (
                    Observation::unavailable(
                        DataSource::LinuxDbus,
                        "the system D-Bus connection is unavailable",
                    ),
                    HardwareErrorKind::Unavailable,
                    "power-profiles-daemon detection could not connect to the system bus",
                ),
                ProbeFailure::PermissionDenied => (
                    Observation::permission_denied(
                        DataSource::LinuxDbus,
                        "access to the power-profiles-daemon D-Bus properties was denied",
                    ),
                    HardwareErrorKind::PermissionDenied,
                    "power-profiles-daemon detection was denied",
                ),
                ProbeFailure::Timeout => (
                    Observation::error(
                        DataSource::LinuxDbus,
                        "power-profiles-daemon did not answer before the read timeout",
                    ),
                    HardwareErrorKind::ReadFailure,
                    "power-profiles-daemon detection timed out",
                ),
                ProbeFailure::MissingProperty(property) => (
                    Observation::error(
                        DataSource::LinuxDbus,
                        format!("required D-Bus property {property} is unavailable"),
                    ),
                    HardwareErrorKind::Unavailable,
                    "a required power-profiles-daemon property is unavailable",
                ),
                ProbeFailure::InvalidProperty(property) => (
                    Observation::error(
                        DataSource::LinuxDbus,
                        format!("required D-Bus property {property} has an invalid value or type"),
                    ),
                    HardwareErrorKind::InvalidData,
                    "a power-profiles-daemon property has an invalid value or type",
                ),
                ProbeFailure::ReadFailure => (
                    Observation::error(
                        DataSource::LinuxDbus,
                        "power-profiles-daemon properties could not be read",
                    ),
                    HardwareErrorKind::ReadFailure,
                    "power-profiles-daemon properties could not be read",
                ),
            };
            (
                observation,
                Some(HardwareError::new(
                    HardwareComponent::Capability,
                    kind,
                    DataSource::LinuxDbus,
                    detail,
                )),
            )
        }
    };

    (
        CapabilityInfo {
            kind: CapabilityKind::PowerProfilesDaemon,
            available,
        },
        issue,
    )
}

#[cfg(target_os = "linux")]
impl PowerProfilesProbe for SystemPowerProfilesProbe {
    fn probe(&self) -> Result<Option<PowerProfilesEvidence>, ProbeFailure> {
        use std::collections::HashMap;

        use zbus::blocking::{Proxy, connection};
        use zbus::zvariant::OwnedValue;

        let connection = connection::Builder::system()
            .map_err(classify_zbus_error)?
            .method_timeout(METHOD_TIMEOUT)
            .build()
            .map_err(classify_zbus_error)?;
        let bus = zbus::blocking::fdo::DBusProxy::new(&connection).map_err(classify_zbus_error)?;
        let service = SERVICE
            .try_into()
            .map_err(|_| ProbeFailure::InvalidProperty("service"))?;
        if !bus
            .name_has_owner(service)
            .map_err(|error| classify_zbus_error(error.into()))?
        {
            return Ok(None);
        }

        let proxy =
            Proxy::new(&connection, SERVICE, PATH, INTERFACE).map_err(classify_zbus_error)?;
        let profiles: Vec<HashMap<String, OwnedValue>> = proxy
            .get_property(READ_PROPERTIES[0])
            .map_err(|error| classify_property_error(error, READ_PROPERTIES[0]))?;
        let version: String = proxy
            .get_property(READ_PROPERTIES[1])
            .map_err(|error| classify_property_error(error, READ_PROPERTIES[1]))?;
        let active_profile: String = proxy
            .get_property(READ_PROPERTIES[2])
            .map_err(|error| classify_property_error(error, READ_PROPERTIES[2]))?;
        if version.trim().is_empty() {
            return Err(ProbeFailure::InvalidProperty(READ_PROPERTIES[1]));
        }
        if active_profile.trim().is_empty() {
            return Err(ProbeFailure::InvalidProperty(READ_PROPERTIES[2]));
        }

        Ok(Some(PowerProfilesEvidence {
            profile_count: profiles.len(),
        }))
    }
}

#[cfg(not(target_os = "linux"))]
impl PowerProfilesProbe for SystemPowerProfilesProbe {
    fn probe(&self) -> Result<Option<PowerProfilesEvidence>, ProbeFailure> {
        Err(ProbeFailure::Connection)
    }
}

#[cfg(target_os = "linux")]
fn classify_property_error(error: zbus::Error, property: &'static str) -> ProbeFailure {
    match error {
        zbus::Error::MethodError(name, _, _) if name.as_str().contains("UnknownProperty") => {
            ProbeFailure::MissingProperty(property)
        }
        zbus::Error::Variant(_)
        | zbus::Error::InvalidReply
        | zbus::Error::MissingField
        | zbus::Error::InvalidField => ProbeFailure::InvalidProperty(property),
        error => classify_zbus_error(error),
    }
}

#[cfg(target_os = "linux")]
fn classify_zbus_error(error: zbus::Error) -> ProbeFailure {
    use std::io::ErrorKind;

    use zbus::Error;

    match error {
        Error::InputOutput(error) | Error::Connection(error, _)
            if error.kind() == ErrorKind::TimedOut =>
        {
            ProbeFailure::Timeout
        }
        Error::InputOutput(error) | Error::Connection(error, _)
            if error.kind() == ErrorKind::PermissionDenied =>
        {
            ProbeFailure::PermissionDenied
        }
        Error::Connection(_, _) => ProbeFailure::Connection,
        Error::MethodError(name, _, _) if name.as_str().contains("AccessDenied") => {
            ProbeFailure::PermissionDenied
        }
        Error::Variant(_) | Error::InvalidReply | Error::MissingField | Error::InvalidField => {
            ProbeFailure::ReadFailure
        }
        _ => ProbeFailure::ReadFailure,
    }
}

#[cfg(test)]
mod tests {
    use adaptive_domain::ObservationStatus;

    use super::*;

    struct FakeProbe(Result<Option<PowerProfilesEvidence>, ProbeFailure>);

    impl PowerProfilesProbe for FakeProbe {
        fn probe(&self) -> Result<Option<PowerProfilesEvidence>, ProbeFailure> {
            self.0.clone()
        }
    }

    #[test]
    fn valid_response_produces_positive_detection_with_evidence() {
        let (capability, issue) =
            capability_from_probe(&FakeProbe(Ok(Some(PowerProfilesEvidence {
                profile_count: 3,
            }))));
        assert_eq!(capability.available.value(), Some(&true));
        assert_eq!(capability.available.status(), ObservationStatus::Available);
        assert_eq!(capability.available.source(), DataSource::LinuxDbus);
        assert!(capability.available.detail().is_some_and(|detail| {
            detail.contains("3 entries") && detail.contains("ActiveProfile")
        }));
        assert!(issue.is_none());
    }

    #[test]
    fn absent_service_is_a_proven_false_value() {
        let (capability, issue) = capability_from_probe(&FakeProbe(Ok(None)));
        assert_eq!(capability.available.value(), Some(&false));
        assert_eq!(capability.available.status(), ObservationStatus::Available);
        assert!(issue.is_none());
    }

    #[test]
    fn connection_error_is_unavailable_without_false_value() {
        let (capability, issue) = capability_from_probe(&FakeProbe(Err(ProbeFailure::Connection)));
        assert_eq!(
            capability.available.status(),
            ObservationStatus::Unavailable
        );
        assert!(capability.available.value().is_none());
        assert_eq!(
            issue.map(|issue| issue.kind),
            Some(HardwareErrorKind::Unavailable)
        );
    }

    #[test]
    fn permission_and_timeout_preserve_distinct_statuses() {
        let (denied, _) = capability_from_probe(&FakeProbe(Err(ProbeFailure::PermissionDenied)));
        let (timeout, _) = capability_from_probe(&FakeProbe(Err(ProbeFailure::Timeout)));
        assert_eq!(
            denied.available.status(),
            ObservationStatus::PermissionDenied
        );
        assert_eq!(timeout.available.status(), ObservationStatus::Error);
        assert!(denied.available.value().is_none());
        assert!(timeout.available.value().is_none());
    }

    #[test]
    fn missing_and_invalid_properties_are_errors_without_values() {
        for failure in [
            ProbeFailure::MissingProperty("Version"),
            ProbeFailure::InvalidProperty("Profiles"),
        ] {
            let (capability, issue) = capability_from_probe(&FakeProbe(Err(failure)));
            assert_eq!(capability.available.status(), ObservationStatus::Error);
            assert!(capability.available.value().is_none());
            assert!(issue.is_some());
        }
    }

    #[test]
    fn interface_exposes_only_the_approved_read_properties() {
        assert_eq!(READ_PROPERTIES, ["Profiles", "Version", "ActiveProfile"]);
    }

    #[test]
    #[ignore = "requires a reachable Linux system bus; absence of the service is valid"]
    fn real_system_bus_probe_is_bounded_and_read_only() {
        let result = SystemPowerProfilesProbe.probe();
        assert!(matches!(result, Ok(Some(_)) | Ok(None)));
    }
}
