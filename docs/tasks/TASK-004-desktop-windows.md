# TASK-004 — Primer escritorio Windows NEXUS

Estado: IMPLEMENTACIÓN CONDICIONADA A COMPILACIÓN Y CI; aceptación formal pendiente.

## Alcance autorizado
- Interfaz Tauri 2 / React / TypeScript para Windows 11, con mediciones solicitadas manualmente.
- Comandos Rust Tauri fijos que reutilizan `adaptive-agent` como biblioteca: CPU, memoria, alimentación, recomendaciones e inventario resumido.
- Inventario filtrado por lista explícita de campos (estados y recuentos). Nunca enviar nombres, seriales ni identificadores a la interfaz.
- Sin ejecutor de shell, sidecars, acceso de archivos, historial, red de telemetría, privilegios elevados ni acciones de optimización.
- Nuevo proyecto Cargo independiente en `apps/desktop/src-tauri` y lockfile propio, sin alterar el workspace Rust original ni su MSRV 1.88.
- Node/npm y Rust stable requeridos sólo para esta interfaz. No se alteran TASK-001/002/003.

## Validación
- `npm ci --ignore-scripts`, `npm audit --audit-level=high`, `npm run build`.
- `cargo test` y `cargo check` con `--locked` sobre el manifiesto del escritorio; compilar un ejecutable de escritorio sin bundle cuando el entorno lo permita.
- CI Rust histórica 4/4 y B4 smoke Linux 2/2 permanecen intactos; nuevo workflow Windows Desktop CI por separado.
- Validar ventana real de forma manual antes de aceptación final. Un build no demuestra que WebView2 funcione en todos los equipos.

## Límites
Primera entrega pre-alpha. Sin optimización automática, monitoreo residente, métricas sostenidas, estimación energética ni instalaciones MSI. Los umbrales de TASK-003 siguen siendo heurísticos; las ausencias se muestran como no disponibles.
## Validacion manual de Windows 2026-11-03
- El propietario confirma que la GUI real abre y que el boton manual `Actualizar todo` muestra resultados en procesador, memoria, alimentacion, inventario y recomendaciones.
- Se conserva `Actualizar ahora` por seccion, sin sondeo residente, acciones automaticas o telemetria remota.
- La compilacion de escritorio de produccion debe realizarse desde `apps/desktop` con `npm run tauri -- build --no-bundle`, no solo con `cargo build --release`: la CLI incorpora los archivos del frontend.
- Desktop CI verificara la compilacion de produccion y la existencia de `nexus-desktop.exe` en Windows, sin generar instaladores.
- Esta verificacion manual no supone la aceptacion formal de TASK-004; la nueva CI debe comprobarse por commit publicado.
