# ADR-0001: Rust workspace modular

- Estado: Aceptada provisionalmente
- Fecha: 2026-10-30

## Contexto

El producto necesita un motor de bajo consumo, contratos compartidos y adaptadores separados para Windows y Linux.

## Decisión

Usar Rust Edition 2024 y un Cargo workspace. Los crates se separan por dominio y responsabilidad. El frontend futuro se comunicará con el agente mediante IPC tipado.

## Consecuencias

- Un solo `Cargo.lock` y comandos de validación compartidos.
- Dependencias explícitas entre crates.
- Código específico del SO aislado en adaptadores.
- La selección final de bibliotecas de hardware se hará en TASK-001 tras verificar MSRV, licencia y cobertura real.

Referencia: https://doc.rust-lang.org/cargo/reference/workspaces.html
