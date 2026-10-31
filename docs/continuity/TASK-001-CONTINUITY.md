# Continuidad operativa — TASK-001 Hardware Discovery

## 1. Sesión y objetivo

- Fecha: 31 de octubre de 2026.
- Proyecto: NEXUS — Adaptive Computer Engine.
- Tarea: TASK-001 — Hardware Discovery read-only.
- Repositorio: `/home/rodrigo/Escritorio/Codigos/NEXUS`.
- Objetivo de esta sesión: continuar la implementación después de HD-AUD,
  completar la detección D-Bus autorizada, corregir la compilación Windows,
  ampliar pruebas y validar Rust 1.88/stable sin alterar Git.

Este documento registra resultados ejecutados. No sustituye el código, los ADR
ni una validación nativa Windows.

## 2. Identidad Git y preservación del trabajo

- Rama inicial y final: `main`.
- HEAD inicial y final: `252d4439e60b243a7728505cfe379a670fb82227`
  (`252d443 docs: record approved TASK-001 hardware discovery specification`).
- Estado inicial: 23 archivos rastreados modificados y 13 rutas nuevas; diff
  acumulado informado de 1.601 inserciones y 100 eliminaciones.
- Staging inicial: vacío.
- Remotos iniciales: ninguno.
- No se ejecutaron `git add`, `commit`, `reset`, `restore`, `checkout`, `clean`
  ni `push`; tampoco se crearon ramas o remotos.
- Los cambios preexistentes se conservaron. Las cifras finales deben leerse en
  la sección 15, generada tras la verificación final de esta sesión.

## 3. Archivos afectados en esta sesión

### Código, manifiesto y lockfile

- `crates/domain/src/observation.rs`
  - añade `DataSource::LinuxDbus`;
  - añade `Observation::available_with_detail` para adjuntar evidencia
    sanitizada a una observación disponible.
- `crates/platform-linux/Cargo.toml`
  - añade `zbus = "=5.19.0"` solo bajo `cfg(target_os = "linux")`;
  - desactiva features por defecto y activa únicamente `async-io` y
    `blocking-api`.
- `crates/platform-linux/src/power_profiles.rs` (nuevo)
  - detector read-only de power-profiles-daemon;
  - timeout de dos segundos;
  - abstracción `PowerProfilesProbe` para pruebas;
  - clasificación de ausencia, conexión, permiso, timeout, propiedad ausente y
    dato/tipo inválido;
  - pruebas unitarias y prueba real ignorada por defecto.
- `crates/platform-linux/src/lib.rs`
  - integra el detector en las capacidades y propaga una incidencia
    estructurada cuando D-Bus falla;
  - mantiene TLP y auto-cpufreq como `unavailable` sin valor falso.
- `crates/platform-linux/tests/inventory.rs`
  - actualiza la invariante: power-profiles-daemon puede ser `true` o `false`
    solo con evidencia D-Bus; errores no contienen valor; TLP/auto-cpufreq
    siguen `unavailable`.
- `crates/platform-windows/src/provider.rs`
  - corrige el import de `StorageMediumTechnology`, fallo descubierto al
    compilar `cfg(windows)`;
  - introduce `StorageQueries` interno para probar la política WMI sin mezclar
    inventarios;
  - marca como parcial el fallback válido después de fallar la fuente primaria;
  - añade pruebas Windows de primaria, conjunto vacío, fallo/tipo inesperado,
    fallback, doble fallo, propiedades ausentes y captura nativa.
- `Cargo.lock`
  - fija `zbus 5.19.0` y sus dependencias transitivas compatibles con Rust
    1.88; Cargo añadió 57 paquetes a la resolución existente.

### Documentación

- `docs/architecture/hardware-discovery.md`: D-Bus implementado, dependencia,
  MSRV, validación cruzada y limitaciones reales.
- `docs/tasks/TASK-001-hardware-discovery.md`: estado y criterios parciales
  actualizados sin declarar TASK-001 aceptada.
- `docs/compatibility/platform-matrix.md`: separa compilación MSVC de ejecución
  Windows nativa.
- `docs/security/threat-model.md`: controles read-only concretos para D-Bus.
- `docs/continuity/TASK-001-CONTINUITY.md`: creado en esta sesión.

No se modificó `.github/workflows/ci.yml` en esta sesión: ya contiene matriz
Ubuntu/Windows × Rust 1.88/stable y los cuatro comandos obligatorios.

## 4. Decisiones técnicas aplicadas

1. El contrato de `HardwareProvider` continúa síncrono y no exige `Send` ni
   `Sync`; se eligió la API bloqueante de zbus, fuera de cualquier runtime
   async.
2. zbus queda condicionado al target Linux. `cargo tree` confirmó que el
   agente Linux incorpora zbus y no WMI, mientras el agente Windows incorpora
   WMI y no zbus.
3. La conexión usa el bus de sistema y un timeout de método de dos segundos.
4. Se consulta primero `NameHasOwner` para distinguir ausencia demostrada de
   error. Solo después se leen `Profiles`, `Version` y `ActiveProfile`.
5. El inventario no serializa los valores de versión o perfil; conserva una
   evidencia mínima: propietario presente, número de perfiles y propiedades
   legibles.
6. La interfaz implementada no contiene setters ni métodos de modificación.
7. TLP y auto-cpufreq no se detectan mediante archivos, procesos ni APIs
   inventadas. Permanecen `unavailable`.
8. Windows conserva `MSFT_PhysicalDisk` como primaria y `Win32_DiskDrive` como
   fallback. Nunca se mezclan ambas listas.
9. Si la primaria WMI falla y el fallback funciona, el almacenamiento es
   `partial` y conserva la incidencia; una primaria vacía permite un fallback
   `available` sin inventar error.

## 5. Dependencia zbus

- Crate: `zbus`.
- Versión exacta: `5.19.0`.
- Ubicación: dependencia normal exclusiva de Linux en
  `adaptive-platform-linux`.
- Features: `default-features = false`, `async-io`, `blocking-api`.
- Tokio: no añadido.
- MSRV declarado por zbus: Rust 1.87; el proyecto permanece en Rust 1.88.
- Licencia declarada: MIT.
- Motivo: cliente D-Bus tipado para consultar el bus de sistema sin procesos
  externos ni FFI insegura propia.
- Coste: el backend `async-io` incorpora executor/tareas y dependencias
  transitivas como `async-io`, `async-executor`, `async-process`, `blocking`,
  `zvariant` y `tracing`. El código de NEXUS no lanza procesos; la feature forma
  parte del backend requerido por zbus.
- Superficie de ataque mitigada: versión exacta, target Linux, tres propiedades
  allowlisted, timeout y ausencia de setters/métodos arbitrarios.

## 6. Detección power-profiles-daemon

- Servicio: `org.freedesktop.UPower.PowerProfiles`.
- Objeto: `/org/freedesktop/UPower/PowerProfiles`.
- Interfaz: `org.freedesktop.UPower.PowerProfiles`.
- Operaciones: `NameHasOwner` del bus y lectura de `Profiles`, `Version`,
  `ActiveProfile`.
- Servicio con propietario y propiedades válidas: `available(true)` con fuente
  `linux_dbus` y evidencia sanitizada.
- El bus demuestra que no hay propietario: `available(false)` con evidencia.
- Conexión inaccesible: `unavailable`, sin valor.
- Permiso denegado: `permission_denied`, sin valor.
- Timeout, fallo de lectura o respuesta inválida: `error`, sin valor e
  incidencia estructurada.
- TLP y auto-cpufreq: `unavailable`, sin valor.

La captura real de esta sesión encontró el servicio activo, tres perfiles y las
tres propiedades legibles. El JSON no contiene los valores concretos del
perfil ni rutas D-Bus internas.

## 7. Windows/WMI

- `wmi 0.18.4` permanece exacto, target-specific y sin features por defecto.
- Todas las consultas siguen siendo `SELECT` constantes con proyección
  explícita; no existe `SELECT *`.
- `WMIConnection` se crea y usa dentro del worker de captura; no cruza hilos.
- Se corrigió un error de compilación real: faltaba importar
  `StorageMediumTechnology` dentro de `provider.rs`.
- La política de almacenamiento ahora es testeable mediante `StorageQueries`.
- Se añadieron once tests dentro del proveedor Windows (cinco anteriores y seis
  nuevos). Compilan bajo MSVC; no se ejecutaron en este host Linux.
- La prueba `native_provider_validates_cpu_battery_and_storage_invariants`
  consulta WMI real cuando la suite se ejecuta en Windows. Exige CPU utilizable,
  una colección de batería coherente (vacía es válida), al menos un disco y
  consistencia en todas sus observaciones.

## 8. Estado HD-AUD

| Hallazgo | Estado al cierre de esta sesión | Evidencia y límite |
|---|---|---|
| HD-AUD-001 | PARTIAL | power-profiles-daemon detectado positivamente por D-Bus real; ausencia/errores probados. TLP y auto-cpufreq siguen BLOCKED por falta de fuente aprobada. |
| HD-AUD-002 | PARTIAL | proveedor WMI compila con stable/1.88 MSVC y el fallo de import se corrigió; ejecución WMI nativa aún BLOCKED. |
| HD-AUD-003 | PASS | regresión de clasificación I/O pasa en ambas suites Linux. |
| HD-AUD-004 | PASS | regresiones de almacenamiento parcial/desaparición pasan. |
| HD-AUD-005 | PASS | topología contradictoria/incompleta continúa probada. |
| HD-AUD-006 | PASS | procedencia Sysfs/Procfs continúa probada. |
| HD-AUD-007 | PARTIAL | 56 pruebas Linux por toolchain y tests Windows compilados; ejecución Windows/CI todavía BLOCKED. |

TASK-001 no se declara terminada ni aceptada.

## 9. Toolchains y targets instalados

- Host: Linux `7.0.0-42-generic`, x86_64.
- Stable activa: `rustc 1.99.1`, `cargo 1.99.1`.
- MSRV instalada durante la sesión: `rustc 1.88.0`, `cargo 1.88.0`, con
  rustfmt y Clippy.
- Target `x86_64-pc-windows-msvc` instalado para stable y 1.88.
- No disponibles: `link.exe`, PowerShell/pwsh, Wine ni un host Windows.

## 10. Comandos y resultados válidos

Todos los comandos siguientes se ejecutaron desde la raíz del repositorio.

| Comando | Plataforma/toolchain | Resultado real |
|---|---|---|
| `cargo info zbus@5.19.0` | Linux | PASS, exit 0; licencia MIT, MSRV 1.87 y features verificadas. |
| `cargo check -p adaptive-platform-linux --all-targets` | Linux stable | PASS, exit 0; actualizó `Cargo.lock`. |
| `cargo test -p adaptive-platform-linux --locked power_profiles -- --nocapture` | Linux stable | PASS, exit 0; 6 pass, 1 ignored. |
| `cargo test -p adaptive-platform-linux --locked real_system_bus_probe_is_bounded_and_read_only -- --ignored --nocapture` | Linux stable, D-Bus real | PASS, exit 0; 1 pass. |
| `cargo +stable fmt --all -- --check` | Linux stable | PASS, exit 0. |
| `cargo +stable check --workspace --all-targets --locked` | Linux stable | PASS, exit 0. |
| `cargo +stable test --workspace --locked` | Linux stable | PASS, exit 0; 56 pass, 1 ignored. |
| `cargo +stable clippy --workspace --all-targets --locked -- -D warnings` | Linux stable | PASS, exit 0. |
| `cargo +1.88.0 fmt --all -- --check` | Linux Rust 1.88 | PASS, exit 0. |
| `cargo +1.88.0 check --workspace --all-targets --locked` | Linux Rust 1.88 | PASS, exit 0. |
| `cargo +1.88.0 test --workspace --locked` | Linux Rust 1.88 | PASS, exit 0; 56 pass, 1 ignored. |
| `cargo +1.88.0 clippy --workspace --all-targets --locked -- -D warnings` | Linux Rust 1.88 | PASS, exit 0. |
| `cargo +stable check --workspace --all-targets --locked --target x86_64-pc-windows-msvc` | cross MSVC | PASS tras corregir import, exit 0. |
| `cargo +stable clippy --workspace --all-targets --locked --target x86_64-pc-windows-msvc -- -D warnings` | cross MSVC | PASS, exit 0. |
| `cargo +1.88.0 check --workspace --all-targets --locked --target x86_64-pc-windows-msvc` | cross MSVC | PASS, exit 0. |
| `cargo +1.88.0 clippy --workspace --all-targets --locked --target x86_64-pc-windows-msvc -- -D warnings` | cross MSVC | PASS, exit 0. |
| `cargo tree --locked --target ... -p adaptive-agent` para Linux y Windows | Linux | PASS; aislamiento zbus/WMI confirmado. |
| `cargo +stable run --quiet -p adaptive-agent` canalizado a `python3` | Linux real | PASS, exit 0; JSON/schema/timestamp/privacidad y D-Bus comprobados. |

## 11. Intentos fallidos y correcciones

1. El primer check MSVC terminó con exit 101 porque
   `StorageMediumTechnology` no estaba importado en `provider.rs`. Se corrigió y
   los cuatro checks/Clippy cruzados pasan.
2. La primera suite Linux posterior a D-Bus tuvo un test fallido: esperaba que
   power-profiles-daemon siempre fuera `unavailable`. Se corrigió la expectativa
   para aceptar `true`/`false` solo con evidencia D-Bus y exigir ausencia de
   valor en errores.
3. La primera ejecución completa stable llegó a Clippy y terminó con exit 101
   por una conversión `.into()` redundante. Se eliminó y Clippy pasa.
4. `cargo +stable test --workspace --locked --target
   x86_64-pc-windows-msvc --no-run` terminó con exit 101: el host no tiene el
   linker MSVC `link.exe`. Por ello no se generaron ni ejecutaron binarios de
   test Windows.
5. Un intento de usar un directorio temporal para la captura fue rechazado por
   la política del entorno antes de ejecutarse. No creó cambios del proyecto.
6. El intento posterior con `jq` no fue válido porque `jq` no está instalado;
   aunque el shell terminó en 0, ese resultado se descartó. La captura se
   repitió con el parser JSON de Python por stdin y pasó.

## 12. Pruebas reales frente a mocks/fixtures

- Hardware Linux real: PASS; captura completa del agente.
- D-Bus real: PASS; power-profiles-daemon activo, tres perfiles y propiedades
  legibles.
- Mocks D-Bus: PASS; presencia, ausencia, conexión, permiso, timeout, propiedad
  ausente y dato/tipo inválido.
- Fixtures sysfs/procfs: PASS; almacenamiento, topología, batería y errores
  parciales.
- Normalización Windows pura en Linux: PASS.
- Compilación del proveedor/tests WMI bajo `cfg(windows)`: PASS por check/Clippy
  cruzados.
- WMI real y tests Windows ejecutados: BLOCKED/NOT RUN.
- Hardware Windows real y captura anonimizada: NOT RUN.

## 13. Privacidad y read-only

- Captura JSON: esquema 1, `captured_at` común, sin claves `timestamp` por
  observación.
- Escaneo de strings de la captura: no encontró `/home/`, `/sys/`, `/proc/`,
  rutas de usuario Windows, seriales, machine-id, nombre o correo del
  propietario.
- Escaneo de producción: no encontró `Command`, PowerShell, systemctl, sockets
  de red ni clientes HTTP.
- Escaneo D-Bus: no encontró setters ni invocaciones modificadoras.
- Las escrituras de filesystem localizadas están exclusivamente en código
  `#[cfg(test)]` y fixtures temporales.
- Los cinco crates mantienen `#![forbid(unsafe_code)]`.
- No se solicitaron privilegios elevados y no se envió telemetría.

## 14. CI y validaciones bloqueadas

- Workflow local: preparado para Ubuntu/Windows × Rust 1.88/stable.
- CI real: NOT RUN; el repositorio continúa sin remoto/GitHub.
- Windows nativo MSVC: BLOCKED por ausencia de host Windows, Visual Studio
  Build Tools/linker y PowerShell.
- La compilación cruzada no sustituye CI ni prueba WMI/hardware real.
- TLP y auto-cpufreq: BLOCKED hasta aprobar una fuente read-only fiable; no se
  deben detectar ejecutando procesos.

## 15. Estado Git final

Esta sección debe coincidir con los comandos finales de la sesión:

- HEAD: `252d4439e60b243a7728505cfe379a670fb82227`.
- Rama: `main`.
- Staging: vacío.
- Remotos: ninguno.
- No se creó ningún commit.
- Worktree final tras la sesión de cierre nativo: 23 archivos rastreados
  modificados y 16 archivos no rastreados. El diff rastreado acumulado contiene
  2.257 inserciones y 103
  eliminaciones; los archivos no rastreados no forman parte de esa estadística.
- Los archivos nuevos creados durante las dos sesiones de continuidad son
  `crates/platform-linux/src/power_profiles.rs`, este documento y
  `scripts/validate-windows-task001.ps1`. Los otros 13 archivos no rastreados
  ya existían antes de esas sesiones y se conservaron.
- `git diff --check`: PASS, exit 0.

## 16. Riesgos y próximos pasos

1. Ejecutar en Windows 11 nativo, con Rust 1.88 y stable:

   ```powershell
   cargo fmt --all -- --check
   cargo check --workspace --all-targets --locked
   cargo test --workspace --locked
   cargo clippy --workspace --all-targets --locked -- -D warnings
   ```

2. Revisar la prueba WMI nativa, capturar un inventario real anonimizado y
   añadir `docs/examples/hardware-inventory-windows.anonymized.json` solo tras
   revisar privacidad.
3. Ejecutar la matriz CI Ubuntu/Windows cuando el propietario autorice conectar
   el repositorio; un workflow local no equivale a CI ejecutada.
4. Realizar la segunda auditoría HD-AUD con evidencia Windows.
5. Decidir por separado si existe una fuente nativa read-only aprobable para
   TLP/auto-cpufreq. No ampliar TASK-001 unilateralmente.
6. No crear commit hasta autorización explícita del propietario.

## 17. Instrucciones para el siguiente chat

1. Tratar el repositorio como fuente de verdad; verificar Git antes de editar.
2. Leer `AGENTS.md`, este documento, los ADR-0005/0006/0007 y la especificación
   de Hardware Discovery.
3. No rehacer D-Bus ni la arquitectura: continuar desde la validación Windows
   nativa/CI.
4. Preservar staging vacío, HEAD y todos los cambios no confirmados.
5. No declarar TASK-001 completa hasta ejecutar Windows nativo y recibir
   aceptación humana.
6. Actualizar este documento al final de cada sesión.

## 18. Referencias oficiales

- power-profiles-daemon D-Bus API:
  <https://upower.pages.freedesktop.org/power-profiles-daemon/gdbus-org.freedesktop.UPower.PowerProfiles.html>
- zbus 5.19.0: <https://docs.rs/crate/zbus/5.19.0>
- zbus blocking API: <https://docs.rs/zbus/5.19.0/zbus/blocking/>
- Cargo target-specific dependencies:
  <https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html>
- Windows MSVC target:
  <https://doc.rust-lang.org/rustc/platform-support/windows-msvc.html>
- WMI 0.18.4: <https://docs.rs/crate/wmi/0.18.4>
- Rust 1.88 release:
  <https://blog.rust-lang.org/2025/06/26/Rust-1.88.0/>
- sysfs rules:
  <https://docs.kernel.org/6.8/admin-guide/sysfs-rules.html>

## 19. Sesión de cierre nativo solicitada — 31 de octubre de 2026

### 19.1 Objetivo y comprobación del entorno

Se solicitó cerrar la validación WMI nativa, captura Windows, matriz Rust
1.88/stable y revisión HD-AUD. Antes de editar se comprobó:

- repositorio y rama: los esperados;
- HEAD: `252d4439e60b243a7728505cfe379a670fb82227`;
- staging: vacío;
- remotos: ninguno;
- host: Linux `7.0.0-42-generic` x86_64;
- `pwsh`, `powershell.exe`, `cmd.exe`, `wine64` y `link.exe`: no disponibles;
- toolchains Linux stable y 1.88: disponibles;
- target `x86_64-pc-windows-msvc`: disponible para compilación cruzada.

No existe Windows nativo ni una sesión Windows accesible desde este entorno.
Por ello WMI real, ejecución de tests Windows y captura Windows continúan
`BLOCKED/NOT RUN`. No se usó Wine, un mock ni compilación cruzada para afirmar
validación nativa.

### 19.2 Cambios de esta sesión

- `crates/platform-windows/src/provider.rs`
  - la prueba nativa WMI ahora valida CPU, batería y almacenamiento;
  - exige `captured_at`, CPU utilizable, batería coherente, al menos un disco y
    consistencia de cada observación de almacenamiento;
  - las pruebas de primaria vacía/fallida, fallback y errores siguen separadas
    como pruebas controladas, no como evidencia de hardware real.
- `scripts/validate.ps1`
  - acepta `-Toolchain 1.88.0` o `-Toolchain stable`;
  - conserva fmt, check, test y Clippy con `Cargo.lock`.
- `scripts/validate-windows-task001.ps1` (nuevo)
  - se niega a ejecutarse fuera de Windows;
  - ejecuta la matriz completa con Rust 1.88.0 y stable;
  - ejecuta explícitamente los tests de `adaptive-platform-windows` en ambas
    toolchains;
  - realiza una captura real del agente con stable;
  - valida esquema 1, `captured_at` UTC común, ausencia de timestamps por
    observación, CPU, memoria, GPU, batería, almacenamiento y capacidades;
  - comprueba detalles sanitizados, patrones sensibles, consultas sin
    `SELECT *` y ausencia de APIs externas/modificadoras en el adaptador;
  - solo escribe
    `docs/examples/hardware-inventory-windows.anonymized.json` cuando se pasa
    `-WriteAnonymizedCapture` y después de superar privacidad.
  - SHA-256 al cierre de esta sesión:
    `b93db63010a12d030cb3087baf3852cefb2630a6538e01ef64965404ea833052`.
- `docs/continuity/TASK-001-CONTINUITY.md`
  - registra esta sesión, los resultados reales y el bloqueo del entorno.

No se añadieron dependencias ni se cambiaron arquitectura, contratos, consultas
WMI, manifiestos Cargo, `Cargo.lock` o workflow CI en esta sesión.

### 19.3 Comandos ejecutados y resultados

| Comando | Resultado |
|---|---|
| Inspección Git, host, toolchains, targets y herramientas Windows | PASS, exit 0; confirmó host Linux y ausencia de runtime/linker Windows. |
| Primera matriz final con `set -e` | FAIL temprano: `cargo +stable fmt --all -- --check` detectó un ajuste mecánico en el nuevo test; ningún comando posterior de esa matriz se contó como ejecutado. |
| `cargo +stable fmt --all` | PASS, exit 0; aplicó únicamente formato Rust. |
| `cargo +stable fmt --all -- --check` | PASS, exit 0. |
| `cargo +stable check --workspace --all-targets --locked` | PASS, exit 0. |
| `cargo +stable test --workspace --locked` | PASS, exit 0; 56 pass, 1 ignored. |
| `cargo +stable clippy --workspace --all-targets --locked -- -D warnings` | PASS, exit 0. |
| `cargo +1.88.0 fmt --all -- --check` | PASS, exit 0. |
| `cargo +1.88.0 check --workspace --all-targets --locked` | PASS, exit 0. |
| `cargo +1.88.0 test --workspace --locked` | PASS, exit 0; 56 pass, 1 ignored. |
| `cargo +1.88.0 clippy --workspace --all-targets --locked -- -D warnings` | PASS, exit 0. |
| `cargo +stable check --workspace --all-targets --locked --target x86_64-pc-windows-msvc` | PASS, exit 0; incluye compilación del test WMI reforzado. |
| `cargo +stable clippy --workspace --all-targets --locked --target x86_64-pc-windows-msvc -- -D warnings` | PASS, exit 0. |
| `cargo +1.88.0 check --workspace --all-targets --locked --target x86_64-pc-windows-msvc` | PASS, exit 0. |
| `cargo +1.88.0 clippy --workspace --all-targets --locked --target x86_64-pc-windows-msvc -- -D warnings` | PASS, exit 0. |
| `scripts/validate-windows-task001.ps1` | NOT RUN: PowerShell y Windows no están disponibles. |
| Tests WMI nativos y captura Windows | BLOCKED/NOT RUN. |

El nuevo script PowerShell no pudo someterse a parseo/ejecución por PowerShell
en este host; su código fue revisado estáticamente y debe validarse en Windows.

### 19.4 Matriz CI

`.github/workflows/ci.yml` ya contiene:

- `ubuntu-latest` y `windows-latest`;
- Rust `1.88` y `stable`;
- rustfmt y Clippy;
- fmt, check, test y Clippy con `Cargo.lock`.

En `windows-latest`, `cargo test --workspace --locked` compilará y ejecutará el
test WMI nativo reforzado. No se añade subida del inventario como artifact para
evitar publicar datos del runner. El workflow no se ejecutó: GitHub continúa
sin conectar y no hay remoto.

### 19.5 Estado HD-AUD después de esta sesión

| Hallazgo | Estado | Motivo |
|---|---|---|
| HD-AUD-001 | PARTIAL | power-profiles-daemon continúa validado; TLP/auto-cpufreq siguen sin fuente aprobada. |
| HD-AUD-002 | PARTIAL/BLOCKED | proveedor y test reforzado compilan MSVC; WMI real no se ejecutó. |
| HD-AUD-003 | PASS | regresiones Linux pasan con ambas toolchains. |
| HD-AUD-004 | PASS | almacenamiento parcial/desaparición continúa verde. |
| HD-AUD-005 | PASS | topología contradictoria/incompleta continúa verde. |
| HD-AUD-006 | PASS | procedencia Sysfs/Procfs continúa verde. |
| HD-AUD-007 | PARTIAL/BLOCKED | Linux pasa y Windows compila; tests Windows nativos y CI no ejecutados. |

TASK-001 continúa sin estar terminada ni aceptada.

### 19.6 Próximo comando obligatorio en Windows nativo

Desde PowerShell, en la raíz del repositorio y con Rust 1.88.0/stable, rustfmt,
Clippy y Visual Studio Build Tools disponibles:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\validate-windows-task001.ps1 -WriteAnonymizedCapture
```

Si se usa PowerShell 7:

```powershell
pwsh -File .\scripts\validate-windows-task001.ps1 -WriteAnonymizedCapture
```

Debe conservarse la salida completa, revisar el JSON antes de incorporarlo y
actualizar este documento con versiones exactas, conteo de tests, estados WMI,
incidencias y hash del archivo. Solo entonces pueden reevaluarse HD-AUD-002 y
HD-AUD-007.

## 20. Sesión de mantenimiento CI — 31 de octubre de 2026

### 20.1 Objetivo y alcance aprobado

Mantenimiento exclusivo del workflow: actualizar `actions/checkout` y fijar la
imagen Ubuntu. Sin cambios de código Rust, dependencias, `Cargo.lock`,
arquitectura V4.2 ni alcance de TASK-001.

### 20.2 Estado de partida verificado

- Rama: `main`; HEAD inicial: `e751a9d`
  (`feat: implement TASK-001 hardware discovery`).
- Remoto: `origin` → `https://github.com/bel001/NEXUS.git`; upstream
  `origin/main`.
- Ejecución CI de referencia: `41249466079`, cuatro trabajos en verde.
- Las secciones 2, 14, 15 y 19.4 registran sesiones anteriores en las que el
  repositorio no tenía remoto y CI no se había ejecutado. Se conservan como
  registro histórico; el estado vigente es el de esta sección.

### 20.3 Revisión del workflow completo

`.github/workflows/ci.yml` tenía 38 líneas: matriz `ubuntu-latest`/
`windows-latest` × `1.88`/`stable`, `fail-fast: false`, componentes `rustfmt` y
`clippy`, y los cuatro comandos obligatorios con `--locked`. La ejecución
`41249466079` emitió dos anotaciones que motivan los cambios de §20.4:

| Anotación observada | Impacto | Acción |
|---|---|---|
| `actions/checkout@v4` apunta a Node.js 20 y se fuerza a Node.js 24 | Deprecación; el paso puede romperse al retirarse la compatibilidad | Actualizar a `@v7` |
| `ubuntu-latest` migra a Ubuntu 26 desde 2026-10-19 | Baseline no reproducible a mitad de la migración | Fijar `ubuntu-24.04` |

Hallazgos revisados y **no** modificados por estar fuera del alcance aprobado:

1. `windows-latest` sigue sin fijar y tiene el mismo riesgo de rollover que
   Ubuntu. Candidato a una tarea de mantenimiento futura.
2. `dtolnay/rust-toolchain@master` es una referencia móvil: no reproducible byte
   a byte. Es el uso documentado por el proyecto upstream.
3. No existe bloque `permissions:`; el `GITHUB_TOKEN` usa los permisos por
   defecto. Recomendado añadir `permissions: contents: read`.
4. No hay `concurrency` ni `timeout-minutes`: pushes rápidos pueden solapar
   ejecuciones y un cuelgue agotaría el máximo por defecto.
5. Las acciones usan etiqueta mayor, no SHA de commit; el endurecimiento de
   cadena de suministro queda pendiente.

Se mantienen sin cambios por ser correctos: `fail-fast: false`, la matriz de dos
sistemas × dos toolchains, `components: rustfmt, clippy`, el uso consistente de
`--locked` y el orden fmt → check → test → clippy con `-D warnings`.

### 20.4 Cambios aplicados

- `.github/workflows/ci.yml`
  - `actions/checkout@v4` → `actions/checkout@v7` (última versión `v7.0.1`;
    etiqueta `v7` → commit `3d3c42e5aac5ba805825da76410c181273ba90b1`).
  - `ubuntu-latest` → `ubuntu-24.04`, con comentario que documenta el motivo y
    advierte de no volver a `ubuntu-latest` sin revisar la migración a Ubuntu 26.
- `docs/continuity/TASK-001-CONTINUITY.md`: esta sección.

No se modificaron código Rust, `Cargo.toml`, `Cargo.lock`, consultas WMI,
contratos, ADR ni la arquitectura V4.2.

### 20.5 Validaciones locales ejecutadas

Todas desde la raíz del repositorio, con `Cargo.lock` versionado.

| Comando | Resultado real |
|---|---|
| Sintaxis YAML + aserciones de matriz | PASS; `os: [ubuntu-24.04, windows-latest]`, `checkout@v7`, sin `ubuntu-latest` ni `@v4` |
| `cargo +stable fmt --all -- --check` | PASS, exit 0 |
| `cargo +stable check --workspace --all-targets --locked` | PASS, exit 0 |
| `cargo +stable test --workspace --locked` | PASS, exit 0; 56 pass, 1 ignored |
| `cargo +stable clippy --workspace --all-targets --locked -- -D warnings` | PASS, exit 0 |
| `cargo +1.88.0 fmt --all -- --check` | PASS, exit 0 |
| `cargo +1.88.0 check --workspace --all-targets --locked` | PASS, exit 0 |
| `cargo +1.88.0 test --workspace --locked` | PASS, exit 0; 56 pass, 1 ignored |
| `cargo +1.88.0 clippy --workspace --all-targets --locked -- -D warnings` | PASS, exit 0 |
| `cargo +stable check`/`clippy --target x86_64-pc-windows-msvc` | PASS, exit 0 |
| `cargo +1.88.0 check`/`clippy --target x86_64-pc-windows-msvc` | PASS, exit 0 |
| `./scripts/validate.sh` | PASS, exit 0 |

El conteo 56/1 es idéntico al baseline documentado: el mantenimiento no alteró
la cobertura.

### 20.6 Ejecución CI de este commit

La verificación de los cuatro trabajos disparados por el commit de mantenimiento
se registra en §20.7, en el commit de documentación inmediatamente posterior. No
se declara verificada antes de leer el resultado real.

### 20.7 Ejecución CI verificada

- Ejecución: `41251868418`, disparada por el push de `49e4f40` a `main`.
- Resultado global: **success**. Cuatro trabajos en verde.

| Trabajo | ID | Duración | Tests |
|---|---|---|---|
| `validate (ubuntu-24.04, 1.88)` | 124686894523 | 1m 1s | 56 passed, 0 failed, 1 ignored |
| `validate (ubuntu-24.04, stable)` | 124686894759 | 44s | 56 passed, 0 failed, 1 ignored |
| `validate (windows-latest, 1.88)` | 124686894735 | 1m 41s | 47 passed, 0 failed, 1 ignored |
| `validate (windows-latest, stable)` | 124686894752 | 1m 29s | 47 passed, 0 failed, 1 ignored |

- Anotaciones emitidas: **ninguna**. Desaparecen la deprecación de Node.js 20 de
  `actions/checkout@v4` y el aviso de migración de `ubuntu-latest` a Ubuntu 26.
- Los nombres de trabajo confirman el pin: `ubuntu-24.04` sustituye a
  `ubuntu-latest`.
- Cobertura WMI nativa: en ambos trabajos Windows se ejecutó
  `provider::tests::native_provider_validates_cpu_battery_and_storage_invariants`
  con resultado `ok`. La suite `adaptive-platform-windows` ejecutó 24 pruebas en
  Windows frente a 13 en Linux.
- Diferencia 56 (Linux) frente a 47 (Windows): en Linux el adaptador ejecuta 34
  pruebas (20 unitarias + 14 de integración); en Windows ejecuta 14 unitarias y
  la integración queda fuera por `cfg`. El proveedor Windows aporta 11 pruebas
  que solo existen con WMI.

### 20.8 Estado HD-AUD actualizado

| Hallazgo | Estado | Evidencia y límite |
|---|---|---|
| HD-AUD-001 | PARTIAL | power-profiles-daemon validado por D-Bus real. TLP y auto-cpufreq siguen sin fuente read-only aprobada. |
| HD-AUD-002 | PARTIAL (avanzado) | WMI real ejecutado en ambos trabajos Windows de CI mediante el test nativo. Falta la captura del equipo Windows 11 del propietario. |
| HD-AUD-003 | PASS | regresiones Linux verdes en ambas toolchains de CI. |
| HD-AUD-004 | PASS | almacenamiento parcial/desaparición verde. |
| HD-AUD-005 | PASS | topología contradictoria/incompleta verde. |
| HD-AUD-006 | PASS | procedencia Sysfs/Procfs verde. |
| HD-AUD-007 | PASS | CI ejecutada en Linux y Windows con 1.88 y stable; la ejecución nativa Windows deja de estar bloqueada. |

TASK-001 **no** se declara terminada ni aceptada. Siguen pendientes:

1. la captura real del equipo Windows 11 del propietario y su JSON anonimizado;
2. la resolución de HD-AUD-001, sin fuente read-only aprobada para TLP ni
   auto-cpufreq;
3. la segunda auditoría HD-AUD con evidencia de hardware Windows real.

### 20.9 Estado Git final

- Commit de mantenimiento: `49e4f40ffaaeea5c6c8080bb136fce978c58e4d8`
  (`ci: bump checkout to v7 and pin ubuntu-24.04 runner`).
- Push: `e751a9d..49e4f40 main -> main` hacia `origin/main`, fast-forward. Sin
  `--force` y sin reescritura de historial.
- Archivos del commit de mantenimiento: `.github/workflows/ci.yml` y
  `docs/continuity/TASK-001-CONTINUITY.md`.
- Commit de documentación: el que contiene esta sección (§20.7–§20.10).
- Sin cambios en código Rust, `Cargo.toml`, `Cargo.lock`, ADR ni arquitectura.
- Los `AGENTS.md` de la sesión de documentación anterior permanecen fuera de
  estos commits y sin modificar.

### 20.10 Próximos pasos

1. Ejecutar `scripts/validate-windows-task001.ps1 -WriteAnonymizedCapture` en
   Windows 11 nativo y revisar el JSON antes de incorporarlo.
2. Resolver HD-AUD-001 solo con una fuente read-only aprobada para TLP y
   auto-cpufreq; no detectar por procesos ni por archivos de unidad.
3. Mantenimiento del workflow pendiente, fuera de esta tarea: fijar
   `windows-latest`, añadir `permissions: contents: read`, `concurrency` y
   `timeout-minutes`, y evaluar el pin por SHA de las acciones.
4. No declarar TASK-001 aceptada hasta la validación del propietario.
