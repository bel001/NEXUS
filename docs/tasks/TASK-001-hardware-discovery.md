# TASK-001 — Hardware Discovery read-only

- Estado: CI y validación física Windows verificadas; documentación y aceptación final pendientes
- Prioridad: P0
- Plataforma: Windows 11 + Linux x86_64
- Tipo: solo lectura
- Aprobación: el propietario aprobó la especificación V4 con las aclaraciones
  V4.1 y V4.2 para preparar su implementación.

La especificación vinculante está en
`docs/architecture/hardware-discovery.md`. La implementación actual no se
considera terminada hasta que pase CI Windows/Linux, MSRV 1.88 y la revisión
del propietario.

La corrección posterior a la primera auditoría resolvió los defectos locales de
errores Linux, topología/procedencia CPU, incidencias de almacenamiento y
normalización Windows. La antigua detección de gestores energéticos por
existencia de archivos se retiró. power-profiles-daemon se detecta ahora
mediante lectura D-Bus nativa con timeout y evidencia explícita; TLP y
auto-cpufreq siguen `unavailable` por falta de una fuente read-only aprobada. No se
declara TASK-001 aceptada por estas correcciones locales.

## Objetivo

Implementar un primer inventario fiable del equipo que identifique hardware y capacidades sin modificar el sistema y sin requerir privilegios elevados por defecto.

## Alcance funcional

Descubrir, cuando esté disponible:

1. Sistema operativo, versión y arquitectura.
2. Fabricante y modelo del equipo.
3. CPU: marca/modelo, núcleos físicos y lógicos si la fuente lo permite.
4. Memoria física total.
5. GPU(s): nombre y fabricante, best effort.
6. Discos: modelo/tipo/capacidad cuando la API lo exponga.
7. Batería: presencia, porcentaje y estado básico cuando corresponda.
8. Capacidades: qué sensores/API están disponibles y qué permisos faltan.

## Fuera de alcance

- Temperaturas, ventiladores y voltajes.
- Cambiar perfiles de energía.
- EcoQoS.
- Escribir en sysfs o registro.
- Ejecutar PowerShell/shell para aplicar cambios.
- Cerrar o priorizar procesos.
- Limpieza de RAM.
- IA.
- UI Tauri.
- Envío de telemetría.

## Contrato de datos

Cada `Observation<T>` debe tener:
- `value` opcional y tipado;
- unidad cuando aplique;
- `status` (`available`, `partial`, `unsupported`, `unavailable`,
  `permission_denied` o `error`);
- `source` o procedencia;
- detalle opcional sanitizado.

`Observation<T>` no tiene timestamp propio. `HardwareInventory` contiene un
único `captured_at` opcional que representa el instante común de captura. Se
serializa en UTC RFC 3339 cuando el reloj civil está disponible, en coherencia
con ADR-0005.

Un valor ausente nunca se convierte en `0`, cadena vacía engañosa o valor inventado.

## Restricciones

- No pedir admin/root para la detección normal.
- No escribir archivos de sistema, registro, sysfs ni configuración.
- No instalar servicios ni drivers.
- No enviar datos fuera del equipo.
- Errores parciales no deben impedir devolver el resto del inventario.
- Evitar dependencias sin revisar licencia, mantenimiento, MSRV y coste.

## Pruebas requeridas

- Fixtures para Windows, Linux, desktop sin batería y laptop con batería.
- Campos no soportados.
- Permiso denegado.
- Fallo parcial de un proveedor.
- Valores desconocidos no representados como cero.
- Tests unitarios del normalizador.
- CI `cargo check`, `cargo test`, `cargo clippy` en Windows y Ubuntu.

## Criterios de aceptación

- [x] El mismo contrato de salida funciona en Windows y Linux; CI y captura Windows física verificadas.
- [x] La detección no modifica el sistema.
- [x] No requiere admin/root en el flujo normal validado en Linux.
- [x] Devuelve disponibilidad por campo.
- [x] Maneja fallos parciales.
- [x] No hay llamadas de red.
- [x] Tests automatizados pasan en CI Windows y Ubuntu, con Rust 1.88 y stable.
- [x] README documenta limitaciones conocidas.
- [ ] Se adjunta ejemplo JSON real anonimizado de cada plataforma.
- [x] Se informa el consumo del proceso durante una sesión de descubrimiento.
- [x] Criterio de gestores energéticos acotado por decisión expresa del propietario: power-profiles-daemon se detecta por D-Bus; TLP y auto-cpufreq se mantienen unavailable, sin afirmar detección positiva. Limitación aceptada para TASK-001.

Validación local del 30 de octubre de 2026: captura real Linux Mint
22.3, JSON validado, aproximadamente 2.5 MiB de RSS máximo y
0.01 s de ejecución caliente. Es una medición puntual, no un benchmark.
El ejemplo Linux anonimizado se conserva en
`docs/examples/hardware-inventory-linux.anonymized.json`.

Posteriormente se validó D-Bus real y la compilación MSVC.
La matriz CI Ubuntu/Windows con Rust 1.88 y stable pasó.

El 1 de noviembre de 2026 se ejecutó el proveedor WMI en el
Windows 11 físico del propietario. Se verificaron CPU, memoria,
GPU, batería, almacenamiento, esquema JSON y cero incidencias.
La captura Windows se mantiene local y sin versionar.

La limitación de TLP y auto-cpufreq como `unavailable` fue
aceptada expresamente para TASK-001. Esto no equivale a
detección positiva de esos gestores.

La aceptación final de TASK-001 permanece pendiente.

## Entrega esperada

Un comando del agente en modo diagnóstico que emita un snapshot JSON normalizado, con datos reales disponibles y estados explícitos para lo que no se pudo obtener.

## Orden de implementación aprobado

1. Modelos de `adaptive-domain` y `Observation<T>`.
2. Contrato `HardwareProvider` y orquestación.
3. Adaptador Linux.
4. Adaptador Windows.
5. Integración con `adaptive-agent`.
6. Serialización JSON versionada.
7. Pruebas, fixtures y CI en Linux y Windows.

Se conservan `adaptive-domain`, `adaptive-hardware-discovery`,
`adaptive-platform-linux`, `adaptive-platform-windows` y `adaptive-agent`.

## Definición de terminado

Ver `docs/tasks/definition-of-done.md`.

## Actualización de aceptación — 1 de noviembre de 2026

- Windows 11 físico: captura WMI real verificada, no simulada.
- Rust 1.88 y stable: pruebas Windows satisfactorias.
- CPU, memoria, GPU, batería y almacenamiento disponibles.
- JSON `schema_version = 1`, timestamp común e incidencias vacías.
- SHA-256 del validador confirmado tras normalizar CRLF a LF.
- HD-AUD-001: alcance reducido con aprobación expresa del propietario.
- TLP y auto-cpufreq: `unavailable` sin valor positivo o negativo inventado.
- HD-AUD-002: PASS técnico sobre hardware físico.
- El ejemplo Windows permanece local y sin versionar hasta autorizar su publicación.
- La revisión de privacidad es preliminar; no garantiza anonimato absoluto.
- TASK-001 no se declara terminada hasta la aceptación final de la entrega y la resolución del ejemplo pendiente de versionar.
