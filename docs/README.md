# Documentación del proyecto

Orden recomendado de lectura:

1. `architecture/overview.md`
2. `architecture/modules.md`
3. `architecture/telemetry-and-policy.md`
4. `architecture/power-management.md`
5. `architecture/hardware-discovery.md`
6. `security/threat-model.md`
7. `compatibility/platform-matrix.md`
8. `decisions/` (ADRs)
9. `tasks/TASK-001-hardware-discovery.md`

La captura real Linux anonimizada está en
`examples/hardware-inventory-linux.anonymized.json`. No existe todavía un
ejemplo equivalente obtenido de hardware Windows real.

Una ADR registra una decisión arquitectónica y su contexto. Las decisiones pueden cambiar, pero deben cambiar mediante otra ADR, no por modificaciones silenciosas.

## Estado de TASK-001

El diseño de Hardware Discovery está aprobado y consolidado en
`architecture/hardware-discovery.md`. Los ADR-0005, ADR-0006 y ADR-0007
registran sus decisiones principales. La implementación de los cinco crates,
la salida JSON, las fixtures y la matriz CI están preparadas. La validación
local en Linux Mint 22.3 pasó; Windows, Rust 1.88 y la ejecución de CI siguen
pendientes y no se consideran validados por las pruebas Linux.
