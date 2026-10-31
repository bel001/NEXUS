# Hardware Discovery — especificación técnica de TASK-001

## 1. Estado y alcance

- Diseño: aprobado por el propietario para implementación.
- Implementación: no iniciada.
- Baseline: `cb8df06`.
- Plataformas: Windows 11 y Linux x86_64.
- Nivel de acción: L0, lectura local.
- MSRV provisional: Rust 1.88.

Este documento consolida el diseño V4 y las aclaraciones V4.1 y V4.2. Es la
referencia técnica de `TASK-001`; no acredita funcionalidad implementada ni
validación en hardware real.

El objetivo es producir un inventario normalizado y versionado. Una fuente
ausente o un componente que falle no debe impedir devolver el resto. Quedan
fuera temperaturas, ventiladores, voltajes, cambios de energía,
optimizaciones, control de procesos, UI, telemetría remota y cualquier
escritura en el sistema.

## 2. Arquitectura y dependencias internas

Se conservan los cinco paquetes existentes:

- `adaptive-domain`: modelos independientes del SO.
- `adaptive-hardware-discovery`: `HardwareProvider` y orquestación.
- `adaptive-platform-linux`: adquisición específica de Linux.
- `adaptive-platform-windows`: adquisición específica de Windows.
- `adaptive-agent`: adaptador nativo y salida JSON.

```text
adaptive-domain
    ^       ^
    |       +----------------------------------+
    |                                          |
adaptive-hardware-discovery                    |
    ^                         ^                |
    |                         |                |
adaptive-platform-linux   adaptive-platform-windows
             ^                  ^
             +--------+---------+
                      |
                adaptive-agent
            (solo adaptador del target)
```

`HardwareProvider` pertenece a `adaptive-hardware-discovery`. Cada adaptador
depende de `adaptive-domain` y `adaptive-hardware-discovery`; el orquestador no
depende de adaptadores concretos. `adaptive-domain` no se reexporta desde el
orquestador. El agente usa `cfg(target_os = "...")` y dependencias por target
para incorporar solo el adaptador nativo. No se permiten ciclos y los cinco
miembros deben continuar compilando en CI.

## 3. `HardwareProvider` y resultados parciales

El contrato ofrece una captura completa de solo lectura. Devuelve un fragmento
normalizado del inventario, incidencias estructuradas y el estado global. Debe
distinguir fallo fatal del proveedor, fallo de una familia de datos,
observación individual ausente e inventario parcial utilizable.

El orquestador recibe el proveedor por inyección y no conoce detalles del SO.
El contrato no expone tipos Win32, objetos WMI, rutas sysfs ni identificadores
internos de correlación.

`Observation<T>` contiene:

- `value: Option<T>`;
- estado `available`, `partial`, `unsupported`, `unavailable`,
  `permission_denied` o `error`;
- fuente tipada;
- detalle opcional sanitizado.

Un dato desconocido nunca se transforma en cero, cadena vacía o estimación no
declarada. `partial` conserva los datos válidos de una observación compuesta.
Los errores de un componente no eliminan los demás.

El tiempo no se duplica en cada observación. `HardwareInventory` contiene un
único `captured_at`, UTC RFC 3339, opcional si no está disponible el reloj
civil.

## 4. Inventario versionado

`HardwareInventory` contiene `schema_version`, `captured_at`, sistema y equipo,
CPU, RAM, GPU, almacenamiento, batería, capacidades e incidencias.

`schema_version` identifica el contrato JSON. Añadir campos opcionales
compatibles no exige necesariamente versión mayor; eliminar campos, cambiar su
significado o su representación de forma incompatible sí. Un lector no debe
reinterpretar versiones mayores desconocidas.

No se serializan devpaths, rutas sysfs, identificadores WMI transitorios,
números de serie ni identificadores personales innecesarios.

## 5. Sistema, equipo y arquitecturas

Se distinguen:

- `binary_architecture`: arquitectura del binario;
- `kernel_architecture`: arquitectura comunicada por el kernel;
- `os_bitness`: bits del SO cuando la fuente lo demuestra;
- `processor_architecture`: arquitectura del procesador según una fuente
  específica.

En Linux, `uname(2).machine` alimenta `kernel_architecture`, no la arquitectura
física. `/etc/os-release` proporciona distribución y versión; DMI ofrece
fabricante y modelo best-effort.

En Windows, `Win32_OperatingSystem.OSArchitecture` describe el SO y puede
estar localizado. `Win32_Processor.Architecture` describe el procesador;
`AddressWidth` no lo sustituye.

## 6. CPU, RAM y GPU

CPU incluye fabricante, modelo, arquitectura, paquetes observables,
procesadores lógicos online y `online_physical_core_count` cuando pueda
demostrarse.

En Linux, `core_cpus_list` es la fuente moderna y `thread_siblings_list` su
fallback, no otra fuente independiente. Los grupos se validan e intersectan
con CPU online. `physical_package_id = -1` y `core_id = 0` pueden ser valores
predeterminados y no bastan para demostrar núcleos físicos. Una topología
incompleta produce estado desconocido o parcial.

En Windows, `Win32_Processor.Architecture` se normaliza así:

| Código | Arquitectura |
|---:|---|
| 0 | x86 |
| 1 | MIPS |
| 2 | Alpha |
| 3 | PowerPC |
| 5 | ARM |
| 6 | Itanium |
| 9 | x64 |
| 12 | ARM64 |
| Otro | Unknown con incidencia |

`NumberOfCores` y `NumberOfLogicalProcessors` se agregan entre procesadores
distintos. No se suman filas duplicadas; si no pueden distinguirse con
evidencia fiable, el resultado es parcial.

RAM representa capacidad física visible en bytes. Linux usa
`/proc/meminfo`; Windows la propiedad documentada del sistema. Un valor ausente
no se convierte en cero. TASK-001 no inventa topología DIMM o canales.

GPU es best-effort, admite múltiples entradas y conserva fabricante, modelo y
fuente. Linux utiliza DRM/sysfs legible y Windows la consulta WMI predefinida.
No se recogen temperaturas, frecuencias o uso. Una lista vacía disponible se
diferencia de una fuente no soportada o no legible.

## 7. Almacenamiento

`StorageDevice` mantiene observaciones independientes para:

- `medium_technology`, `transport`, `nature` y `presentation`;
- `removability`, `rotation` y `supports_removable_media`;
- `capacity_bytes`;
- `reported_sector_size_bytes`;
- `logical_sector_size_bytes`;
- `physical_sector_size_bytes`.

Medio extraíble, dispositivo físicamente extraíble y hot-plug son conceptos
diferentes. `/sys/class/block/<disk>/removable` y
`Win32_DiskDrive.Capabilities = 7` solo sustentan
`supports_removable_media`. `MediaType` describe el medio utilizado o
accesible, no removibilidad física.

`Win32_DiskDrive.BytesPerSector` se registra como
`reported_sector_size_bytes`; tamaños lógico y físico permanecen ausentes sin
una fuente aprobada.

## 8. Batería

Una batería puede estar legítimamente ausente. Se separan presencia,
porcentaje, dirección (`charging`, `discharging`, `not_charging`, `unknown`) y
nivel (`full`, `high`, `low`, `critical`, `partial`, `unknown`).

Normalización de `Win32_Battery.BatteryStatus`:

| Código | Significado | Dirección | Nivel |
|---:|---|---|---|
| 1 | Other | Unknown | Unknown |
| 2 | Unknown | Unknown | Unknown |
| 3 | Fully Charged | Not charging | Full |
| 4 | Low | Unknown | Low |
| 5 | Critical | Unknown | Critical |
| 6 | Charging | Charging | Unknown |
| 7 | Charging and High | Charging | High |
| 8 | Charging and Low | Charging | Low |
| 9 | Charging and Critical | Charging | Critical |
| 10 | Undefined | Unknown | Unknown |
| 11 | Partially Charged | Unknown | Partial |
| Otro | No reconocido | Unknown | Unknown con incidencia |

Low y Critical no implican Discharging.

## 9. Estrategia Linux

Solo se leen `/proc`, `/etc/os-release`, sysfs y `uname(2)`. No se ejecutan
herramientas externas ni se escribe en sysfs.

Conforme al ADR-0004, las capacidades Linux incluyen la detección read-only de
gestores energéticos activos conocidos. TASK-001 no cambia perfiles, no compite
por controles y no convierte esta observación en una acción de energía.

### Block moderno

Si `/sys/subsystem` existe, `/sys/subsystem/block/devices` es la única vista.
No se consultan simultáneamente `/sys/class/block` ni `/sys/block`. Cada
entrada se resuelve bajo `/sys/devices` y las particiones se excluyen por el
atributo `partition` del destino.

Raíz block ausente produce `unavailable`; acceso denegado,
`permission_denied`; fallo de lectura, `error`; raíz legible vacía, colección
disponible vacía. No hay fallback legacy cuando `/sys/subsystem` existe.

### Block legacy

Solo si `/sys/subsystem` no existe se inspeccionan todas las vistas aplicables:
`/sys/class/block`, `/sys/block` y una exposición real de block bajo `/sys/bus`
si existe. No se elige únicamente la primera ruta.

Las vistas forman una unión tras resolver a `/sys/devices`. Una vista ausente
no es error; una vista existente que falla genera resultado parcial si otra
aportó datos.

### Resolución, deduplicación y particiones

- la jerarquía física se examina exclusivamente bajo `/sys/devices`;
- el devpath canónico es una clave efímera de una captura;
- dos enlaces al mismo destino producen un dispositivo;
- destinos distintos no se fusionan por nombre, modelo o capacidad;
- particiones detectadas por `partition` se excluyen;
- enlaces rotos, bucles, destinos externos y desapariciones generan
  incidencias por entrada;
- el devpath no se serializa, persiste ni registra.

El enlace `device` no es la ruta normal. Solo se usa como recuperación
condicional de un layout antiguo cuando la entrada de clase no resuelve bajo
`/sys/devices`. Su resultado también debe resolver y validarse bajo
`/sys/devices`; de lo contrario queda no disponible.

### Removibilidad física

`/sys/devices/.../removable` es best-effort y depende del bus o subsistema:

| Situación | Resultado |
|---|---|
| Atributo ausente/no soportado | `unavailable` |
| `unknown` | `available(Unknown)` |
| `fixed` | `available(Fixed)` |
| `removable` | `available(Removable)` |
| Acceso denegado | `permission_denied` |
| Texto inválido/fallo | `error` |

Se recorren padres reales identificados por subsistema y se selecciona el
objeto compatible más próximo que ofrezca el atributo. No se busca en
ancestros arbitrarios. USB o PCI no determinan el valor. Un USB sin soporte no
hereda `fixed` del controlador PCI superior.

## 10. Estrategia Windows y WMI

WMI se limita a consultas `SELECT` constantes con propiedades enumeradas. No
acepta fragmentos de UI o IA, no usa `SELECT *` y no invoca métodos. Cada
conexión se crea y utiliza en el mismo hilo; ninguna conexión, interfaz COM u
objeto WMI cruza esa frontera.

Para almacenamiento:

1. `MSFT_PhysicalDisk` es la fuente primaria.
2. `Win32_DiskDrive` es fallback si la primaria no está disponible o falla
   como inventario.
3. No se mezclan inventarios sin correlación fiable.
4. No se correlaciona por nombres comerciales.
5. Identificadores de correlación fiables son transitorios y no serializados.

`MSFT_PhysicalDisk.SpindleSpeed`: `0` es NonRotational,
`1..UINT32_MAX-1` Rotational y `UINT32_MAX` Unknown. No determina
`medium_technology`. Si contradice `MediaType`, se conservan ambas fuentes y
se genera incidencia sin forzar reconciliación.

La tabla `BusType` es la específica de `MSFT_PhysicalDisk`. No se traslada la
tabla de `STORAGE_BUS_TYPE`: los valores 18–20 no se reinterpretan como SCM,
UFS o NVMe-oF sin evidencia de la interfaz. Los códigos no reconocidos quedan
Unknown con incidencia.

## 11. Seguridad y privacidad

Hardware Discovery es estrictamente read-only:

- no modifica registro, sysfs, configuración, servicios o energía;
- no instala controladores o servicios;
- no ejecuta shell, PowerShell ni procesos auxiliares;
- no solicita administrador/root en el flujo normal;
- no ejecuta optimizaciones;
- no usa red ni telemetría;
- no recopila contenido personal ni identificadores innecesarios;
- limita y sanitiza incidencias antes de serializarlas.

## 12. Dependencias y MSRV

MSRV provisional: Rust 1.88.

| Dependencia | Uso | Configuración acordada |
|---|---|---|
| `serde` | Modelos e intercambio tipado | `derive` |
| `serde_json` | JSON en ejecución | Dependencia normal de `adaptive-agent` |
| `time` | UTC y RFC 3339 | `0.3.55`, `serde-well-known`, sin `local-offset` |
| `rustix` | `uname(2)` seguro | Solo Linux, feature mínima de sistema |
| `wmi` | Consultas WMI | `0.18.4`, `default-features = false`, sin `chrono` ni `time` |
| `tempfile` | Fixtures temporales de Linux | `dev-dependency` exclusiva de `adaptive-platform-linux` |

Antes de cambiar manifiestos se comprobarán licencia, mantenimiento,
dependencias transitivas, tamaño y compatibilidad efectiva con Rust 1.88. No
se requiere runtime asíncrono ni una dependencia para ejecutar comandos.

`tempfile` se utiliza únicamente para pruebas y fixtures temporales del
adaptador Linux. Nunca es una dependencia normal de producción y no se
incorpora a `adaptive-domain`, `adaptive-hardware-discovery`,
`adaptive-platform-windows` ni `adaptive-agent`.

## 13. Orden de implementación

1. Modelos de `adaptive-domain` y `Observation<T>`.
2. `HardwareProvider` y orquestador.
3. Adaptador Linux.
4. Adaptador Windows.
5. Integración con `adaptive-agent`.
6. Serialización JSON versionada.
7. Pruebas, fixtures y CI.

Cada etapa debe mantener el workspace compilable y añadir sus pruebas.

## 14. Pruebas y CI

Se requieren pruebas unitarias, proveedores falsos, serialización, fixtures y
pruebas reales condicionadas por plataforma.

Linux cubre modo moderno/legacy, vistas duplicadas y complementarias,
particiones, enlaces rotos, bucles, destinos externos, desaparición durante la
captura, `removable` ausente/unknown/fixed/removable, PCI sin soporte y USB sin
herencia desde PCI.

Windows cubre allowlist de consultas, confinamiento al hilo, primaria/fallback
sin mezcla, CPU múltiples/duplicadas, códigos desconocidos y contradicción
entre rotación y tecnología.

CI ejecutará Linux y Windows con Rust 1.88 y stable, utilizando `Cargo.lock`:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Compilar en Linux no valida Windows. La aceptación requiere ejemplos JSON
reales anonimizados por plataforma y coste documentado de una captura.

## 15. Limitaciones

- sysfs, WMI, DMI, DRM y batería dependen del equipo, firmware, controladores,
  contenedor y permisos.
- GPU y almacenamiento son best-effort.
- La topología CPU puede no demostrar todos los núcleos instalados.
- La removibilidad puede no estar soportada aunque se conozca el transporte.
- WMI puede devolver valores localizados, incompletos o contradictorios.
- No se declara compatibilidad universal ni con kernels no probados.
- Rust 1.88 es provisional hasta validar las versiones fijadas en Cargo.lock.
- Diseño aprobado no equivale a implementación o validación funcional.
