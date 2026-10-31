# Módulos y límites

| Módulo | Responsabilidad | No debe hacer |
|---|---|---|
| domain | Tipos, enums, contratos y reglas puras | Llamar APIs del SO |
| hardware-discovery | Orquestar inventario read-only | Cambiar configuración |
| platform-windows | Consultar interfaces Windows | Exponer tipos Win32 al dominio |
| platform-linux | Consultar interfaces Linux | Escribir sysfs en MVP |
| telemetry | Recoger métricas y snapshots | Tomar decisiones de optimización |
| policy-engine | Clasificar contexto y proponer políticas | Ejecutar acciones |
| action-engine (futuro) | Ejecutar acciones declarativas autorizadas | Aceptar shell arbitrario |
| rollback (futuro) | Guardar/restaurar estado | Restaurar valores que no cambió |
| benchmark (futuro) | Comparar mediciones repetibles | Inventar métricas o prometer mejoras |
| storage (futuro) | Historial local | Enviar datos a nube |
| agent | Ciclo de vida, IPC y scheduler | Acumular privilegios |
| desktop (futuro) | UI y consentimiento | Acceso directo al sistema |

## Dependencias permitidas

- `domain` no depende de ningún otro crate del producto.
- `platform-*` depende de `domain`.
- `hardware-discovery` depende de `domain` y de abstracciones de plataforma.
- `policy-engine` consume snapshots y produce planes.
- `action-engine` consume planes aprobados y adaptadores explícitos.
- `desktop` se comunica con `agent` mediante contratos versionados.

## Reglas de compilación multiplataforma

Usar módulos y compilación condicional (`cfg(target_os = "windows")`, `cfg(target_os = "linux")`) dentro de adaptadores. El dominio compartido debe compilar sin importar Win32 o bibliotecas Linux.

La documentación de Rust sobre `cfg` define el mecanismo de compilación condicional: https://doc.rust-lang.org/reference/conditional-compilation.html
