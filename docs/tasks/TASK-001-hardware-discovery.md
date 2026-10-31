# TASK-001 — Hardware Discovery read-only

- Estado: Diseño técnico aprobado; implementación no iniciada
- Prioridad: P0
- Plataforma: Windows 11 + Linux x86_64
- Tipo: solo lectura
- Aprobación: el propietario aprobó la especificación V4 con las aclaraciones
  V4.1 y V4.2 para preparar su implementación.

La especificación vinculante está en
`docs/architecture/hardware-discovery.md`. Aprobar el diseño no implica que la
funcionalidad esté implementada o validada.

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

- [ ] El mismo contrato de salida funciona en Windows y Linux.
- [ ] La detección no modifica el sistema.
- [ ] No requiere admin/root en el flujo normal.
- [ ] Devuelve disponibilidad por campo.
- [ ] Maneja fallos parciales.
- [ ] No hay llamadas de red.
- [ ] Tests automatizados pasan en CI Windows y Ubuntu.
- [ ] README documenta limitaciones conocidas.
- [ ] Se adjunta ejemplo JSON real anonimizado de cada plataforma.
- [ ] Se informa el consumo del proceso durante una sesión de descubrimiento.

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
