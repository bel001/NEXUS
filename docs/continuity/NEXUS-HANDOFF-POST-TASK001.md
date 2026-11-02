# NEXUS — Traspaso de contexto posterior a TASK-001

## Estado confirmado

- Proyecto: NEXUS — Adaptive Computer Engine.
- Rama principal: `main`.
- Repositorio: https://github.com/bel001/NEXUS
- Arquitectura: V4.2; cinco paquetes Rust.
- TASK-001 Hardware Discovery: ACEPTADA por el propietario.
- Fecha de aceptación: 2026-11-01.
- Commit técnico/documental validado: `3cea5a259fea34c8d01e8696ccf2a1320a87c0ab`.
- Última CI verificada antes del cierre: Rust CI #6.
- GitHub Actions run: `41376861193`, cuatro trabajos PASS.
- El commit de este cierre documental debe comprobarse por separado.
- TASK-002: NO INICIADA.

## Contexto histórico

El PDF `NEXUS_contexto_traspaso_TASK001.pdf` corresponde
al 30 de octubre de 2026 y al commit `252d443`.
Su estado Git, ausencia de implementación y ausencia de
GitHub son datos históricos, no el estado actual.

Consultar los documentos actuales del repositorio antes
de tomar decisiones.

## Arquitectura y contratos

- `crates/domain`: tipos, observaciones y reglas puras.
- `crates/hardware-discovery`: HardwareProvider y orquestación.
- `crates/platform-linux`: adquisición Linux.
- `crates/platform-windows`: adquisición Windows.
- `apps/agent`: ejecución nativa y salida JSON.

- Contrato HardwareInventory versionado, esquema 1.
- Un único `captured_at` opcional por inventario.
- Observaciones con estados y fuentes explícitos.
- Los errores parciales conservan datos válidos.
- No inventar valores desconocidos.
- Respetar ADR-0001 a ADR-0007.

## Evidencia de TASK-001

- Rust 1.88 y stable verificados.
- CI Windows y Ubuntu: cuatro combinaciones PASS.
- Windows 11 físico: WMI nativo validado.
- Linux real: captura y consulta D-Bus validadas.
- Ejemplos JSON reales anonimizados de ambas plataformas.
- Windows utiliza MSFT_PhysicalDisk con fallback Win32_DiskDrive.
- Linux usa sysfs/procfs y D-Bus según las fuentes aprobadas.
- TLP y auto-cpufreq permanecen unavailable por limitación aceptada.
- No se declara compatibilidad universal de hardware.

## Privacidad y seguridad

- Hardware Discovery estrictamente read-only.
- Sin cambios de configuración ni privilegios innecesarios.
- Sin telemetría o conexiones remotas del agente.
- Sin ejecución de herramientas externas para descubrimiento.
- Sin números de serie, UUID, MAC o identificadores personales.
- La captura Windows original permanece local y privada.
- El ejemplo público reducido no garantiza anonimato absoluto.
- No modificar servicios, perfiles de energía ni dispositivos.

## Git y trabajo futuro

- Comprobar siempre `git status --short --branch`.
- Comprobar HEAD, remoto y CI antes de nuevos cambios.
- Preservar los commits y el trabajo local.
- No usar reset, clean, restore, checkout destructivos ni force push.
- No hacer commits ni push sin autorización expresa.
- El commit documental de cierre debe registrarse a continuación.
- La licencia del proyecto sigue pendiente de decisión.

## Siguiente fase

TASK-001 está cerrada tras aceptación del propietario.
TASK-002 puede planificarse solamente como una nueva tarea,
con alcance, riesgos y criterios de aceptación propios.

No asumir que TASK-002 ya tiene un diseño aprobado.
No añadir UI, SQLite, IA, optimización energética ni
actuaciones sobre el sistema sin su especificación aprobada.

Fuentes de verdad:
- docs/tasks/definition-of-done.md
- docs/tasks/TASK-001-hardware-discovery.md
- docs/architecture/hardware-discovery.md
- docs/compatibility/platform-matrix.md
- docs/continuity/TASK-001-CONTINUITY.md
- docs/continuity/TASK-001-HD-AUD-SECOND-REVIEW.md
