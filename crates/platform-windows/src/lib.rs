#![forbid(unsafe_code)]

//! Adaptador Windows read-only. La lógica pura se compila en todas las
//! plataformas; WMI solo se incorpora bajo `cfg(target_os = "windows")`.

#[cfg(target_os = "windows")]
pub mod cpu_telemetry;
#[cfg(target_os = "windows")]
pub mod memory_telemetry;
pub mod normalization;
#[cfg(any(target_os = "windows", test))]
mod queries;

#[cfg(target_os = "windows")]
mod provider;

#[cfg(target_os = "windows")]
pub use provider::WindowsHardwareProvider;
