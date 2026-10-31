# ADR-0006: Descubrimiento Linux mediante vistas sysfs resueltas

- Estado: Aceptada para implementación
- Fecha: 2026-10-30

## Contexto

Sysfs separa vistas de clasificación y jerarquía física. Los layouts modernos
y legacy pueden exponer block en ubicaciones diferentes, y varias vistas
legacy pueden referirse al mismo dispositivo.

## Decisión

Con `/sys/subsystem` se usa exclusivamente
`/sys/subsystem/block/devices`. Sin `/sys/subsystem` se inspecciona la unión de
las vistas legacy aplicables, no solo la primera ruta existente.

Todo candidato se resuelve a `/sys/devices`; el destino canónico es una clave
efímera de deduplicación. Se excluyen particiones, no se exponen devpaths y los
errores por entrada producen resultados parciales.

El enlace `device` solo puede usarse como recuperación condicionada de un
layout antiguo. La removibilidad física es best-effort y nunca se infiere solo
por pertenencia a USB o PCI.

## Consecuencias

- Las vistas duplicadas no duplican discos.
- Una vista legacy incompleta no oculta dispositivos de otra.
- La ausencia de `removable` es `unavailable`, no fixed.
- Se requieren filesystem inyectable y fixtures de enlaces.

Las reglas completas están en
`docs/architecture/hardware-discovery.md`.

## Referencia

- https://docs.kernel.org/6.8/admin-guide/sysfs-rules.html
