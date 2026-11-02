//! Motor determinista de recomendaciones. No accede al SO ni ejecuta acciones.
use adaptive_domain::telemetry::{CpuScope, PowerSource, TelemetrySnapshot, TelemetryStatus};
use serde_json::{Value, json};

pub const POLICY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, PartialEq, Eq)]
pub struct Recommendation {
    pub code: &'static str,
    pub explanation: &'static str,
    pub expected_impact: &'static str,
    pub risk: &'static str,
    pub action: &'static str,
    pub requirements: &'static [&'static str],
}

impl Recommendation {
    fn as_value(&self) -> Value {
        json!({
            "code": self.code,
            "explanation": self.explanation,
            "expected_impact": self.expected_impact,
            "risk": self.risk,
            "action": self.action,
            "requirements": self.requirements,
        })
    }
}

#[derive(Debug)]
pub struct PolicyReport {
    recommendations: Vec<Recommendation>,
    unavailable_evidence: Vec<&'static str>,
}

impl PolicyReport {
    pub fn as_value(&self) -> Value {
        json!({
            "schema_version": POLICY_SCHEMA_VERSION,
            "mode": "read_only",
            "recommendations": self.recommendations.iter().map(Recommendation::as_value).collect::<Vec<_>>(),
            "unavailable_evidence": self.unavailable_evidence,
            "limitations": ["single_on_demand_snapshot", "no_sustained_load_inference", "no_energy_measurement"],
        })
    }
}

/// Conservative output when collectors produce an inconsistent combined snapshot.
pub fn invalid_snapshot_report() -> PolicyReport {
    PolicyReport {
        recommendations: Vec::new(),
        unavailable_evidence: vec!["invalid_combined_snapshot"],
    }
}

/// Pure evaluation: absent, partial, or unknown readings never become zero.
/// Thresholds are triage heuristics, not certified performance budgets.
pub fn evaluate(snapshot: &TelemetrySnapshot) -> PolicyReport {
    let mut report = PolicyReport {
        recommendations: Vec::new(),
        unavailable_evidence: Vec::new(),
    };

    let cpu = snapshot.cpu_usage();
    if cpu.cpu_scope() == CpuScope::SystemTotal
        && cpu.usage_percent().status() == TelemetryStatus::Available
        && cpu.sample_window_ms().is_some_and(|ms| ms > 0)
    {
        if cpu.usage_percent().value().is_some_and(|v| *v >= 85.0) {
            report.recommendations.push(Recommendation {
                code: "cpu_sample_elevated",
                explanation: "La muestra global de CPU fue alta; un instante no demuestra carga sostenida.",
                expected_impact: "informational",
                risk: "none_read_only",
                action: "read_only",
                requirements: &["repeat_user_requested_measurement"],
            });
        }
    } else {
        report.unavailable_evidence.push("cpu_system_total");
    }

    let total = snapshot.memory_total_visible_bytes();
    let available = snapshot.memory_available_bytes();
    match (total.value(), available.value()) {
        (Some(&total_bytes), Some(&available_bytes))
            if total.status() == TelemetryStatus::Available
                && available.status() == TelemetryStatus::Available
                && total_bytes > 0
                && available_bytes <= total_bytes =>
        {
            if (available_bytes as u128) * 100 <= (total_bytes as u128) * 10 {
                report.recommendations.push(Recommendation {
                    code: "memory_available_low",
                    explanation: "La memoria disponible fue baja en esta lectura; no identifica procesos responsables.",
                    expected_impact: "informational",
                    risk: "none_read_only",
                    action: "read_only",
                    requirements: &["confirm_with_another_user_requested_reading"],
                });
            }
        }
        _ => report
            .unavailable_evidence
            .push("memory_total_and_available"),
    }

    let power = snapshot.power_source();
    if power.status() != TelemetryStatus::Available {
        report.unavailable_evidence.push("power_source");
    } else if power.value() == Some(&PowerSource::Battery) {
        let charge = snapshot.battery_percent();
        if charge.status() == TelemetryStatus::Available {
            if charge.value().is_some_and(|v| *v <= 20) {
                report.recommendations.push(Recommendation {
                    code: "battery_charge_low",
                    explanation: "La bateria reporta una carga baja; considera conectar alimentacion cuando sea conveniente.",
                    expected_impact: "informational",
                    risk: "none_read_only",
                    action: "read_only",
                    requirements: &["user_decision"],
                });
            }
        } else {
            report.unavailable_evidence.push("battery_percent");
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use adaptive_domain::telemetry::{
        CpuUsageReading, TelemetryQuality, TelemetryReading, TelemetryReason, TelemetrySource,
    };

    fn fixture(
        cpu: Option<f64>,
        memory: Option<(u64, u64)>,
        battery: Option<u8>,
    ) -> TelemetrySnapshot {
        let source = TelemetrySource::Fixture;
        let unavailable_u64 =
            || TelemetryReading::<u64>::unavailable(source, TelemetryReason::NotCollected);
        let cpu = match cpu {
            Some(value) => {
                CpuUsageReading::available_now(value, 250, CpuScope::SystemTotal, source)
                    .expect("valid test CPU")
            }
            None => CpuUsageReading::unavailable(
                source,
                CpuScope::Unknown,
                TelemetryReason::NotCollected,
            ),
        };
        let (total, available) = match memory {
            Some((t, a)) => (
                TelemetryReading::<u64>::bytes_now(t, source, TelemetryQuality::Measured),
                TelemetryReading::<u64>::bytes_now(a, source, TelemetryQuality::Measured),
            ),
            None => (unavailable_u64(), unavailable_u64()),
        };
        let (power, charge) = match battery {
            Some(value) => (
                TelemetryReading::power_now(PowerSource::Battery, source),
                TelemetryReading::<u8>::percent_now(value, source, TelemetryQuality::Measured)
                    .expect("valid test battery"),
            ),
            None => (
                TelemetryReading::unavailable(source, TelemetryReason::NotCollected),
                TelemetryReading::unavailable(source, TelemetryReason::NotCollected),
            ),
        };
        TelemetrySnapshot::new(cpu, total, available, power, charge, vec![])
            .expect("valid test snapshot")
    }

    #[test]
    fn rules_fire_on_fixture_and_do_not_emit_raw_values() {
        let report = evaluate(&fixture(Some(90.0), Some((10_000, 1_000)), Some(15)));
        let codes: Vec<_> = report.recommendations.iter().map(|r| r.code).collect();
        assert_eq!(
            codes,
            [
                "cpu_sample_elevated",
                "memory_available_low",
                "battery_charge_low"
            ]
        );
        let output = report.as_value().to_string();
        assert!(output.contains("read_only"));
        assert!(!output.contains("10000"));
    }

    #[test]
    fn boundary_values_and_normal_snapshot_do_not_trigger() {
        let report = evaluate(&fixture(Some(84.9), Some((10_000, 1_001)), Some(21)));
        assert!(report.recommendations.is_empty());
    }

    #[test]
    fn missing_evidence_is_not_interpreted_as_zero() {
        let report = evaluate(&fixture(None, None, None));
        assert!(report.recommendations.is_empty());
        assert_eq!(
            report.unavailable_evidence,
            [
                "cpu_system_total",
                "memory_total_and_available",
                "power_source"
            ]
        );
    }

    #[test]
    fn cpu_threshold_and_battery_boundary_are_inclusive() {
        let report = evaluate(&fixture(Some(85.0), Some((10_000, 2_000)), Some(20)));
        assert_eq!(report.recommendations.len(), 2);
    }

    #[test]
    fn memory_ratio_is_overflow_safe() {
        let report = evaluate(&fixture(Some(10.0), Some((u64::MAX, 1)), Some(80)));
        assert_eq!(report.recommendations.len(), 1);
        assert_eq!(report.recommendations[0].code, "memory_available_low");
    }

    #[test]
    fn invalid_snapshot_emits_no_recommendations() {
        let report = invalid_snapshot_report();
        assert!(report.recommendations.is_empty());
        assert_eq!(report.unavailable_evidence, ["invalid_combined_snapshot"]);
    }
}
