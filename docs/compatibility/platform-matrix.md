# Matriz de compatibilidad inicial

| Función | Windows 11 | Linux |
|---|---|---|
| OS y arquitectura | Objetivo | Objetivo |
| CPU y núcleos | Objetivo | Objetivo |
| RAM total | Objetivo | Objetivo |
| GPU | Best effort | Best effort |
| Discos | Best effort | Best effort |
| Batería básica | Portátil | Portátil |
| Temperatura | Sensor/driver disponible | hwmon/sensor disponible |
| Perfil energético | API Windows | D-Bus si power-profiles-daemon |
| Cambios energéticos | Fase posterior | Fase posterior |
| Control de ventiladores | No MVP | No MVP |

## Estados de capacidad

- `available`: lectura disponible y validada.
- `partial`: algunos campos disponibles.
- `unsupported`: la plataforma/hardware no expone la función.
- `unavailable`: no se obtuvo dato en esta sesión.
- `permission_denied`: faltan permisos.
- `error`: error técnico; registrar detalle sanitizado.

## Matriz de validación manual inicial

- Windows 11 x64, Intel, laptop con batería.
- Windows 11 x64, AMD, laptop con batería.
- Windows 11 x64, desktop sin batería.
- Ubuntu LTS x64, laptop con batería.
- Ubuntu LTS x64, desktop sin batería.
- Linux con power-profiles-daemon.
- Linux con TLP.
- Linux con ambos paquetes instalados, verificando detección de conflicto.

No se declarará compatibilidad universal hasta probar equipos físicos y documentar limitaciones.
