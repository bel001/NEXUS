# Matriz de compatibilidad inicial

| Función | Windows 11 | Linux |
|---|---|---|
| OS y arquitectura | Validado en Windows 11 físico y CI con 1.88/stable | Validado en Linux Mint 22.3 x86_64 |
| CPU y núcleos | WMI nativo validado en Windows físico | Validado en equipo real y fixtures |
| RAM total | WMI nativo validado en Windows físico | Validado en equipo real y fixtures |
| GPU | GPU integrada detectada en Windows físico | Validado best effort en Intel DRM |
| Discos | NVMe físico detectado; pruebas WMI y fallback aprobadas | Validado con NVMe, USB, loop y zram |
| Batería básica | WMI nativo validado en Windows físico | Validado en portátil y fixtures sin batería |
| Gestores energéticos activos | No aplican gestores Linux en TASK-001 Windows | power-profiles-daemon validado por D-Bus; TLP/auto-cpufreq `unavailable` sin evidencia aprobada |
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

## Estado de TASK-001 — actualización 2026-11-01

Windows 11 físico: WMI nativo validado en portátil x86_64,
CPU con 12 procesadores lógicos y 10 núcleos físicos,
GPU integrada, SSD NVMe y batería real.

La captura generada usa esquema 1, fuente WMI y cero incidencias.
Los campos no demostrables permanecen `unavailable`.

Linux: captura física y D-Bus real previamente validados.
La matriz GitHub Actions Ubuntu/Windows con Rust 1.88/stable
pasó en cuatro combinaciones.

HD-AUD-001: power-profiles-daemon tiene evidencia positiva
D-Bus. Por excepción de alcance aprobada, TLP y auto-cpufreq
quedan `unavailable` sin inferir actividad.

Esta evidencia no demuestra compatibilidad universal con
hardware AMD, otros portátiles ni equipos sin batería.
Se preparó un ejemplo público reducido de Windows en
`docs/examples/hardware-inventory-windows.public.anonymized.json`.
La captura original permanece privada. TASK-001 fue aceptada formalmente el 2026-11-01.
