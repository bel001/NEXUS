# ADR-0005: Contrato de Hardware Discovery e inventario versionado

- Estado: Aceptada para implementación
- Fecha: 2026-10-30

## Contexto

Windows y Linux ofrecen datos incompletos y fuentes diferentes. Un fallo local
no debe invalidar el inventario ni filtrar tipos del sistema operativo al
dominio.

## Decisión

`HardwareProvider` reside en `adaptive-hardware-discovery`. Los adaptadores
dependen de ese contrato y de `adaptive-domain`; el orquestador no depende de
adaptadores concretos.

Los modelos usan `Observation<T>` con estados explícitos y resultados
parciales. `HardwareInventory` tiene `schema_version` y un único `captured_at`
UTC opcional. El inventario no serializa rutas internas ni identificadores de
correlación.

Los detalles están en `docs/architecture/hardware-discovery.md`.

## Consecuencias

- No hay ciclos entre crates.
- El agente inyecta el adaptador nativo.
- Windows y Linux producen el mismo contrato.
- Un cambio JSON incompatible requiere evolucionar el esquema.
- Los datos desconocidos no se representan como cero.
