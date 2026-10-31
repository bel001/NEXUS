# Módulos y límites

| Módulo | Responsabilidad | No debe hacer |
|---|---|---|
| domain | Tipos, enums, observaciones y reglas puras del inventario | Llamar APIs del SO |
| hardware-discovery | Definir `HardwareProvider`, aplicar versión y captura común | Cambiar configuración |
| platform-windows | Consultar WMI local mediante proyecciones fijas | Exponer tipos Win32 al dominio |
| platform-linux | Leer procfs, sysfs, DMI, DRM, `os-release` y `uname` | Escribir sysfs en MVP |
| telemetry | Recoger métricas y snapshots | Tomar decisiones de optimización |
| policy-engine | Clasificar contexto y proponer políticas | Ejecutar acciones |
| action-engine (futuro) | Ejecutar acciones declarativas autorizadas | Aceptar shell arbitrario |
| rollback (futuro) | Guardar/restaurar estado | Restaurar valores que no cambió |
| benchmark (futuro) | Comparar mediciones repetibles | Inventar métricas o prometer mejoras |
| storage (futuro) | Historial local | Enviar datos a nube |
| agent | Seleccionar el adaptador nativo y emitir JSON versionado | Acumular privilegios |
| desktop (futuro) | UI y consentimiento | Acceso directo al sistema |

## Dependencias permitidas

- `domain` no depende de ningún otro crate del producto.
- `hardware-discovery` depende de `domain` y define el contrato de aplicación
  `HardwareProvider`.
- Cada `platform-*` depende directamente de `domain` y de
  `hardware-discovery` para implementar `HardwareProvider`.
- `hardware-discovery` no depende de adaptadores concretos; el agente inyecta
  el adaptador del target y no se permiten ciclos.
- `agent` incorpora solo `platform-windows` en Windows o `platform-linux` en
  Linux mediante dependencias por target.
- `policy-engine` consume snapshots y produce planes.
- `action-engine` consume planes aprobados y adaptadores explícitos.
- `desktop` se comunica con `agent` mediante contratos versionados.

## Reglas de compilación multiplataforma

Usar módulos y compilación condicional (`cfg(target_os = "windows")`, `cfg(target_os = "linux")`) dentro de adaptadores. El dominio compartido debe compilar sin importar Win32 o bibliotecas Linux.

La especificación del grafo y del contrato de TASK-001 está en
`hardware-discovery.md`.

Este grafo está implementado para TASK-001. En Linux el agente incorpora solo
`adaptive-platform-linux`; en Windows incorpora solo
`adaptive-platform-windows`. Los adaptadores continúan como miembros del
workspace para permitir sus validaciones independientes.

La documentación de Rust sobre `cfg` define el mecanismo de compilación condicional: https://doc.rust-lang.org/reference/conditional-compilation.html
