# NEXUS — Segunda auditoría HD-AUD

Fecha: 2026-11-01
Baseline: `3259a22`
Plataformas: Windows 11 físico y Linux previamente validado.

## Resultados

| Hallazgo | Estado | Evidencia |
|---|---|---|
| HD-AUD-001 | LIMITACIÓN ACEPTADA | D-Bus real para power-profiles-daemon; TLP/auto-cpufreq unavailable por decisión del propietario |
| HD-AUD-002 | PASS técnico | Captura física Windows WMI, CPU, batería, almacenamiento y JSON |
| HD-AUD-003 | PASS previo | Clasificación de errores Linux |
| HD-AUD-004 | PASS previo | Conservación de incidencias parciales |
| HD-AUD-005 | PASS previo | Validación de topología CPU |
| HD-AUD-006 | PASS previo | Procedencia Sysfs/Procfs |
| HD-AUD-007 | PASS previo | CI Ubuntu/Windows con 1.88 y stable |

## Evidencia física Windows

- Sistema: Windows 11 Pro x64.
- Procesador: CPU x86_64, 10 núcleos físicos y 12 lógicos.
- Memoria visible: aproximadamente 15.69 GiB.
- GPU: adaptador gráfico integrado detectado mediante WMI.
- Almacenamiento: SSD NVMe físico de aproximadamente 1 TB.
- Batería: detectada; dirección y nivel no inferidos.
- JSON: `schema_version=1`; cero incidencias.
- SHA-256 del script validado tras normalizar LF.
- Los modelos concretos del hardware pueden facilitar
  correlaciones e identificación indirecta.

## Decisión de alcance

El propietario acepta `unavailable` para TLP y auto-cpufreq
dentro de TASK-001. No hay detección positiva de esos gestores.
No deben interpretarse como ausentes ni inactivos.
No se ejecutan procesos auxiliares ni se agregan dependencias.

## Riesgos y pendientes

- Captura original Windows privada; ejemplo público anonimizado preparado en `docs/examples/hardware-inventory-windows.public.anonymized.json`.
- No afirmar compatibilidad universal.
- No publicar la captura original. El ejemplo público conserva un riesgo residual de correlación por características técnicas.
- TASK-001 aceptada formalmente por el propietario el 2026-11-01, después de CI #6.
- TASK-002 no iniciada.

## Comprobaciones de esta sesión
- Rust 1.88.0: fmt PASS
- Rust 1.88.0: check PASS
- Rust 1.88.0: test PASS
- Rust 1.88.0: clippy PASS
- Rust stable: fmt PASS
- Rust stable: check PASS
- Rust stable: test PASS
- Rust stable: clippy PASS
