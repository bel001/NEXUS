# ADR-0002: Windows y Linux desde el inicio

- Estado: Aceptada
- Fecha: 2026-10-30

## Decisión

Windows y Linux son objetivos de primera clase desde el inicio. macOS queda fuera del alcance inicial, pero el dominio común no debe impedir añadirlo después.

## Consecuencias

- Contratos independientes del SO.
- Adaptadores y pruebas separados.
- CI en Windows y Ubuntu.
- Las funciones no disponibles deben representarse explícitamente.
- No se fingirá compatibilidad por compilar en ambos sistemas.
