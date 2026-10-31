# Matriz de compatibilidad inicial

| Función | Windows 11 | Linux |
|---|---|---|
| OS y arquitectura | Compila MSVC con 1.88/stable; ejecución nativa pendiente | Validado en Linux Mint 22.3 x86_64 |
| CPU y núcleos | Compila MSVC; tests nativos pendientes | Validado en equipo real y fixtures |
| RAM total | Compila MSVC; tests nativos pendientes | Validado en equipo real y fixtures |
| GPU | Compila MSVC; validación best effort pendiente | Validado best effort en Intel DRM |
| Discos | Primaria/fallback compilan y sus tests nativos están preparados | Validado con NVMe, USB, loop y zram |
| Batería básica | Compila MSVC; tests nativos pendientes | Validado en portátil y fixtures sin batería |
| Gestores energéticos activos | No aplica en TASK-001 | power-profiles-daemon validado por D-Bus; TLP/auto-cpufreq `unavailable` sin evidencia aprobada |
| Temperatura (fase posterior; fuera de TASK-001) | No TASK-001 | No TASK-001 |
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

## Estado de TASK-001

El diseño técnico y la implementación local están preparados para revisión.
En Linux pasaron compilación, pruebas, Clippy y una captura real read-only.
La lógica pura de normalización Windows se probó en Linux. El código WMI y sus
tests condicionados compilan mediante el target MSVC con Rust 1.88 y stable,
pero no se ejecutaron porque el host no es Windows y carece de `link.exe`.
Compilar o probar una plataforma no valida la otra.

Las correcciones HD-AUD añaden fixtures Linux para sysfs legacy, desaparición,
errores parciales, topología y baterías, además de normalización Windows para
códigos ausentes/desconocidos y evidencia contradictoria. Las pruebas del
proveedor WMI condicionadas a Windows permanecen sin ejecutar localmente. Se
añadieron casos de primaria vacía/fallida, fallback exitoso/fallido,
propiedades ausentes, resultados parciales y captura nativa.

El workflow preparado cubre Windows y Linux con Rust 1.88 y stable, usando
`Cargo.lock`. Todavía no se ha ejecutado porque el repositorio no tiene remoto.
El MSRV se validó localmente en Linux y por check/Clippy cruzado MSVC. La
compatibilidad Windows completa solo se declarará después de ejecutar CI y la
matriz manual en Windows nativo.
