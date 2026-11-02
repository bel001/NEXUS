# Adaptive Computer Engine

> Nombre comercial pendiente. Repositorio de trabajo para una plataforma de optimización adaptativa de PC y laptops.

**Estado:** pre-alpha · **Primera plataforma:** Windows + Linux · **Primera entrega funcional:** Hardware Discovery · **Modo actual:** solo lectura.

## Propósito

Construir una plataforma local que mida el estado del equipo y recomiende o aplique, con autorización y mecanismos de reversión, políticas compatibles para mejorar la respuesta del sistema y la eficiencia energética.

No promete convertir hardware distinto en Apple Silicon ni superar universalmente a una Mac. El objetivo medible es mejorar el aprovechamiento del hardware existente y el rendimiento por vatio sin degradar estabilidad, seguridad o experiencia de uso.

## Decisiones iniciales

- Core y agente: Rust.
- Interfaz futura: Tauri 2 + React + TypeScript.
- Persistencia local: SQLite, cuando el historial sea necesario.
- Windows y Linux desde el inicio mediante adaptadores separados.
- Reglas deterministas primero; IA local solo después de tener telemetría y evaluaciones fiables.
- Sin nube obligatoria ni telemetría remota por defecto.
- El primer incremento técnico es Hardware Discovery en modo estrictamente read-only.

## Estado del repositorio

El workspace contiene la implementación de Hardware Discovery:
contrato versionado, adaptador Linux, adaptador Windows mediante
WMI y salida JSON del agente.

Linux dispone de captura real y validación D-Bus.
Windows dispone de pruebas WMI nativas aprobadas en CI y
sobre el equipo Windows 11 físico del propietario.

Hardware Discovery permanece estrictamente read-only.
TASK-001 todavía requiere aceptación final.

## Requisitos de desarrollo

- Rust 1.88 como MSRV provisional y Rust stable para validación.
- Git.
- Node.js LTS y pnpm/npm solo cuando se incorpore la interfaz Tauri.
- Windows 11 y una distribución Linux de escritorio para validación real.

## Comandos iniciales

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

En PowerShell se pueden ejecutar los mismos comandos. También están disponibles `scripts/validate.sh` y `scripts/validate.ps1`.

Para emitir un inventario JSON en la plataforma nativa:

```bash
cargo run --locked -p adaptive-agent
```

## Primera tarea en revisión

Leer `docs/architecture/hardware-discovery.md`, los ADR-0005 a ADR-0007 y
`docs/tasks/TASK-001-hardware-discovery.md`.

El diseño y el alcance están aprobados. La implementación local está lista
para revisión; la matriz CI y la validación Windows física se completaron.
TASK-001 continúa pendiente de aceptación final.

## Estructura

```text
apps/agent/                 Agente local y salida JSON
crates/domain/              Tipos y contratos compartidos
crates/hardware-discovery/  Orquestación de descubrimiento
crates/platform-windows/    Adaptador Windows
crates/platform-linux/      Adaptador Linux
docs/                       Arquitectura, ADRs, seguridad y tareas
.github/                    CI, plantillas de issues y PR
scripts/                    Validación local
```

## Seguridad

- No ejecutar comandos arbitrarios construidos por el frontend o por IA.
- No desactivar Defender, actualizaciones, firewall ni servicios esenciales.
- No eliminar archivos del sistema ni desactivar el archivo de paginación.
- No limpiar cachés/RAM agresivamente.
- No modificar voltajes, frecuencias, BIOS/UEFI o ventiladores en el MVP.
- Toda acción futura debe tener precondiciones, consentimiento cuando corresponda, verificación y rollback.

Hardware Discovery solo lee interfaces locales documentadas. No ejecuta
comandos auxiliares, no solicita privilegios elevados, no usa red y no
recopila números de serie, UUID, MAC, usuarios o credenciales.

## Licencia

Pendiente de decisión. No publicar ni redistribuir el código como open source hasta elegir y añadir una licencia.

## Validación TASK-001 — noviembre de 2026

- CI Ubuntu 24.04 y Windows, con Rust 1.88 y stable: PASS.
- WMI real: verificado en runners y en Windows 11 físico.
- Hardware Discovery permanece estrictamente read-only.
- power-profiles-daemon: consulta D-Bus validada en Linux.
- TLP y auto-cpufreq: `unavailable` por limitación aprobada.
- Captura Windows anonimizada generada; publicación pendiente.
- TASK-001 todavía requiere aceptación final.
- La licencia del proyecto sigue pendiente de decisión.
