# Prompt de agente — TASK-001 Hardware Discovery

Actúa como ingeniero senior de Rust y sistemas operativos. Implementarás exclusivamente `TASK-001-hardware-discovery.md`.

## Antes de editar

1. Lee `AGENTS.md`, la arquitectura y la tarea.
2. Inspecciona el workspace.
3. Presenta un plan de implementación y lista exacta de archivos que cambiarás.
4. Identifica las dependencias candidatas con versión, licencia, MSRV, mantenimiento y cobertura Windows/Linux.
5. No edites hasta recibir aprobación del plan.

## Implementación autorizada (una vez aprobado)

- Implementar contratos tipados de inventario en `crates/domain`.
- Implementar el orquestador read-only en `crates/hardware-discovery`.
- Implementar proveedores específicos en `crates/platform-windows` y `crates/platform-linux`.
- Añadir fixtures y pruebas.
- Añadir un modo de salida JSON del agente.
- No añadir acciones de optimización ni modificar configuración del sistema.

## Restricciones

- No ejecutar shell arbitrario.
- No requerir admin/root en el camino normal.
- No escribir en registro, sysfs, archivos de sistema o configuración.
- No acceder a contenido personal.
- No enviar red.
- No usar `unwrap()`/`expect()` en rutas operativas.
- No representar valores desconocidos como cero.
- Separar las APIs de cada plataforma mediante módulos/adaptadores.
- Si una API necesita privilegios, devolver `permission_denied` y continuar con los demás campos.

## Entrega

1. Resumen de arquitectura.
2. Archivos creados/modificados.
3. Dependencias y licencias.
4. Ejemplos JSON anonimizados Windows/Linux.
5. Pruebas ejecutadas con resultados reales.
6. Limitaciones conocidas.
7. Riesgos pendientes.
8. Confirmación explícita de que no se modificó el sistema.

Detente al terminar TASK-001. No comiences Telemetry Engine ni Energy Engine sin validación humana.
