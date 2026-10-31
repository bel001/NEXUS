# Definition of Done

Una tarea no está terminada hasta que:

- Cumple el alcance aprobado y no incorpora funciones adicionales.
- Los contratos públicos están documentados.
- Los errores y capacidades no disponibles se representan explícitamente.
- Hay pruebas unitarias y, cuando aplique, integración.
- Se ejecutaron `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo test --workspace` y `cargo clippy --workspace --all-targets -- -D warnings`.
- CI pasa en Windows y Ubuntu.
- No se introducen permisos elevados innecesarios.
- No hay cambios destructivos ni telemetría de red no autorizada.
- Se documentan dependencias, licencias y limitaciones.
- El autor informa qué se probó realmente y qué no.
- El propietario del producto valida la entrega antes de comenzar la siguiente tarea.
