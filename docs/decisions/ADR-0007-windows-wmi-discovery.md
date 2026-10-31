# ADR-0007: Descubrimiento Windows mediante WMI de solo lectura

- Estado: Aceptada para implementación
- Fecha: 2026-10-30

## Contexto

WMI permite obtener el inventario requerido, pero sus conexiones COM tienen
restricciones de hilo y distintas clases describen almacenamiento con
semánticas diferentes.

## Decisión

El adaptador ejecuta únicamente consultas `SELECT` predefinidas, con
propiedades enumeradas y sin métodos. Cada conexión se crea y usa en el mismo
hilo.

`MSFT_PhysicalDisk` es la fuente primaria y `Win32_DiskDrive` el fallback. No
se combinan sin correlación fiable; los identificadores transitorios no se
serializan.

`SpindleSpeed` describe rotación, no tecnología del medio. `MediaType`,
`BytesPerSector`, capacidades y BusType conservan la semántica de su clase.

## Consecuencias

- No se ejecutan PowerShell, comandos externos ni métodos WMI.
- Los tipos WMI quedan confinados al adaptador.
- Fallos y contradicciones se conservan como observaciones e incidencias.
- Se usará `wmi` 0.18.4 con features por defecto desactivadas.

Los detalles están en `docs/architecture/hardware-discovery.md`.
