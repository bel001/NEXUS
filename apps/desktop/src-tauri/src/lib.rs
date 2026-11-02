#![forbid(unsafe_code)]
use serde_json::{Value, json};

fn parse(result: Result<String, serde_json::Error>) -> Result<Value, String> {
    let data = result.map_err(|_| "serialization_failed".to_string())?;
    serde_json::from_str(&data).map_err(|_| "invalid_json".to_string())
}

#[tauri::command]
fn read_cpu() -> Result<Value, String> {
    parse(adaptive_agent::cpu_telemetry_json())
}
#[tauri::command]
fn read_memory() -> Result<Value, String> {
    parse(adaptive_agent::memory_telemetry_json())
}
#[tauri::command]
fn read_power() -> Result<Value, String> {
    parse(adaptive_agent::power_telemetry_json())
}
#[tauri::command]
fn read_recommendations() -> Result<Value, String> {
    parse(adaptive_agent::recommendations_json())
}
/// Only a strict allowlist leaves Rust. No CPU model, device names, paths or serials.
fn summarize_inventory(raw: &Value) -> Value {
    let safe_status = |name: &str| raw.get(name)
        .and_then(|v| v.get("status"))
        .and_then(Value::as_str)
        .filter(|v| matches!(*v, "available" | "unavailable" | "unsupported" | "permission_denied" | "error"))
        .unwrap_or("unavailable")
        .to_string();
    let count = |name: &str| raw.get(name)
        .and_then(|v| v.get("value"))
        .and_then(Value::as_array)
        .map(Vec::len);
    json!({
        "schema_version": 1,
        "cpu_status": safe_status("cpu"),
        "memory_status": safe_status("memory"),
        "gpus_status": safe_status("gpus"),
        "storage_status": safe_status("storage"),
        "batteries_status": safe_status("batteries"),
        "gpu_count": count("gpus"),
        "storage_count": count("storage"),
        "battery_count": count("batteries"),
    })
}
#[tauri::command]
fn read_inventory_summary() -> Result<Value, String> {
    let raw = parse(adaptive_agent::run())?;
    if raw.get("schema_version").and_then(Value::as_u64) != Some(1) {
        return Err("unsupported_inventory_schema".into());
    }
    Ok(summarize_inventory(&raw))
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![read_cpu, read_memory, read_power, read_inventory_summary, read_recommendations])
        .run(tauri::generate_context!())
        .expect("cannot start NEXUS desktop");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inventory_summary_never_exposes_identifiers() {
        let raw = json!({"schema_version":1,"cpu":{"status":"available","value":{"model":"PRIVATE"}},"storage":{"status":"available","value":[{"serial":"SECRET"}]},"issues":[{"detail":"PRIVATE"}]});
        let safe = summarize_inventory(&raw).to_string();
        assert!(!safe.contains("PRIVATE"));
        assert!(!safe.contains("SECRET"));
        assert_eq!(safe_status_field(&safe), Some("available"));
    }
    fn safe_status_field(raw: &str) -> Option<&str> {
        if raw.contains("\"cpu_status\":\"available\"") { Some("available") } else { None }
    }
}
