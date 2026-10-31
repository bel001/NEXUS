#![forbid(unsafe_code)]

//! Contratos compartidos de Hardware Discovery.
//!
//! Este crate no accede al sistema operativo. Define el inventario versionado,
//! observaciones tipadas y errores sanitizados que comparten los adaptadores.

mod inventory;
mod observation;

pub use inventory::*;
pub use observation::*;
