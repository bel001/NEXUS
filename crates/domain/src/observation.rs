use serde::Serialize;

/// Estado explícito de una observación.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationStatus {
    Available,
    Partial,
    Unsupported,
    Unavailable,
    PermissionDenied,
    Error,
}

/// Fuente normalizada. Las rutas reales del sistema nunca se exponen.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSource {
    Orchestrator,
    Runtime,
    LinuxProcfs,
    LinuxSysfs,
    LinuxSysfsBlockModern,
    LinuxSysfsBlockLegacy,
    LinuxOsRelease,
    LinuxUname,
    LinuxDmi,
    LinuxDbus,
    WindowsWmiCimv2,
    WindowsWmiStorage,
}

/// Unidad de un valor cuantitativo.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    Bytes,
    Count,
    Percent,
    BytesPerSector,
    RevolutionsPerMinute,
}

/// Dato tipado con disponibilidad, procedencia y detalle seguro.
///
/// El timestamp pertenece al inventario completo, no a cada observación.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Observation<T> {
    value: Option<T>,
    status: ObservationStatus,
    source: DataSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    unit: Option<Unit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

impl<T> Observation<T> {
    pub fn available(value: T, source: DataSource, unit: Option<Unit>) -> Self {
        Self {
            value: Some(value),
            status: ObservationStatus::Available,
            source,
            unit,
            detail: None,
        }
    }

    /// Dato disponible acompañado por evidencia textual sanitizada.
    pub fn available_with_detail(
        value: T,
        source: DataSource,
        unit: Option<Unit>,
        detail: impl AsRef<str>,
    ) -> Self {
        Self {
            value: Some(value),
            status: ObservationStatus::Available,
            source,
            unit,
            detail: Some(sanitize_detail(detail.as_ref())),
        }
    }

    pub fn partial(
        value: T,
        source: DataSource,
        unit: Option<Unit>,
        detail: impl AsRef<str>,
    ) -> Self {
        Self {
            value: Some(value),
            status: ObservationStatus::Partial,
            source,
            unit,
            detail: Some(sanitize_detail(detail.as_ref())),
        }
    }

    pub fn unsupported(source: DataSource, detail: impl AsRef<str>) -> Self {
        Self::missing(ObservationStatus::Unsupported, source, detail)
    }

    pub fn unavailable(source: DataSource, detail: impl AsRef<str>) -> Self {
        Self::missing(ObservationStatus::Unavailable, source, detail)
    }

    pub fn permission_denied(source: DataSource, detail: impl AsRef<str>) -> Self {
        Self::missing(ObservationStatus::PermissionDenied, source, detail)
    }

    pub fn error(source: DataSource, detail: impl AsRef<str>) -> Self {
        Self::missing(ObservationStatus::Error, source, detail)
    }

    fn missing(status: ObservationStatus, source: DataSource, detail: impl AsRef<str>) -> Self {
        debug_assert!(!matches!(
            status,
            ObservationStatus::Available | ObservationStatus::Partial
        ));
        Self {
            value: None,
            status,
            source,
            unit: None,
            detail: Some(sanitize_detail(detail.as_ref())),
        }
    }

    pub fn value(&self) -> Option<&T> {
        self.value.as_ref()
    }

    pub fn status(&self) -> ObservationStatus {
        self.status
    }

    pub fn source(&self) -> DataSource {
        self.source
    }

    pub fn unit(&self) -> Option<Unit> {
        self.unit
    }

    pub fn detail(&self) -> Option<&str> {
        self.detail.as_deref()
    }

    pub fn is_consistent(&self) -> bool {
        matches!(
            (self.value.is_some(), self.status),
            (
                true,
                ObservationStatus::Available | ObservationStatus::Partial
            ) | (
                false,
                ObservationStatus::Unsupported
                    | ObservationStatus::Unavailable
                    | ObservationStatus::PermissionDenied
                    | ObservationStatus::Error
            )
        )
    }
}

/// Elimina saltos de línea, controles y detalles excesivamente largos.
pub fn sanitize_detail(input: &str) -> String {
    let mut output = String::with_capacity(input.len().min(240));
    let mut previous_space = false;
    for character in input.chars().take(240) {
        let normalized = if character.is_control() {
            ' '
        } else {
            character
        };
        if normalized.is_whitespace() {
            if !previous_space {
                output.push(' ');
            }
            previous_space = true;
        } else {
            output.push(normalized);
            previous_space = false;
        }
    }
    output.trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_preserve_observation_invariants() {
        let available = Observation::available(8_u32, DataSource::Runtime, Some(Unit::Count));
        assert!(available.is_consistent());
        assert_eq!(available.value(), Some(&8));

        let partial = Observation::partial(4_u32, DataSource::Runtime, Some(Unit::Count), "half");
        assert!(partial.is_consistent());
        assert_eq!(partial.status(), ObservationStatus::Partial);

        for missing in [
            Observation::<u32>::unsupported(DataSource::Runtime, "unsupported"),
            Observation::<u32>::unavailable(DataSource::Runtime, "unavailable"),
            Observation::<u32>::permission_denied(DataSource::Runtime, "denied"),
            Observation::<u32>::error(DataSource::Runtime, "error"),
        ] {
            assert!(missing.is_consistent());
            assert!(missing.value().is_none());
        }
    }

    #[test]
    fn details_are_sanitized_and_bounded() {
        let input = format!("secret\npath\t{}", "x".repeat(300));
        let detail = sanitize_detail(&input);
        assert!(!detail.contains('\n'));
        assert!(detail.chars().count() <= 240);
    }
}
