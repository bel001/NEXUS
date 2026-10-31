# ADR-0003: Descubrimiento read-only antes de optimizar

- Estado: Aceptada
- Fecha: 2026-10-30

## Contexto

Un optimizador que modifica parámetros sin conocer el hardware puede causar regresiones, conflictos o inestabilidad.

## Decisión

La primera tarea funcional solo detecta hardware y capacidades. No cambia perfiles, servicios, procesos, archivos, voltajes, frecuencias ni firmware.

## Consecuencias

- Podemos validar el modelo de datos sin riesgo operativo.
- La primera demo no necesita privilegios de administrador/root.
- Las acciones se implementarán solo después de tener telemetría y rollback.
