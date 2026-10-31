# Gestión energética por plataforma

## Windows

Primera etapa: lectura y diagnóstico.
- Consultar estado de alimentación y batería mediante APIs documentadas.
- Identificar plan/perfil actual.
- Usar `powercfg` solo para diagnósticos solicitados y documentados.
- No cambiar planes globales durante Hardware Discovery.

Fase posterior:
- Evaluar EcoQoS para procesos secundarios compatibles.
- Respetar políticas del usuario, aplicaciones foreground y límites térmicos.
- Registrar estado previo y verificar cada operación.
- No forzar frecuencias, voltajes ni planes de energía a ciegas.

Referencias:
- Microsoft Quality of Service / EcoQoS: https://learn.microsoft.com/en-us/windows/win32/procthread/quality-of-service
- Powercfg: https://learn.microsoft.com/en-us/windows-hardware/design/device-experiences/powercfg-command-line-options

## Linux

Primera etapa: detectar sin modificar.
- Identificar si está activo `power-profiles-daemon`, TLP, auto-cpufreq u otro gestor conocido.
- Leer perfil actual cuando exista una API soportada.
- No escribir directamente en `/sys` en el MVP.

Fase posterior:
- Si `power-profiles-daemon` está activo, considerar su API D-Bus y no competir con él.
- Si TLP está activo, tratarlo como gestor externo y no escribir los mismos tunables.
- Si no existe gestor, decidir una estrategia explícita y documentada para la distribución y el hardware.

Referencias:
- power-profiles-daemon D-Bus API: https://upower.pages.freedesktop.org/power-profiles-daemon/gdbus-org.freedesktop.UPower.PowerProfiles.html
- TLP: conflictos con otros gestores: https://linrunner.de/tlp/faq/conflicts.html
- Arquitectura event-driven de TLP: https://linrunner.de/tlp/developers/architecture.html

## Regla de coexistencia

Nunca administrar el mismo control desde dos gestores a la vez. La detección de conflictos es un requisito de seguridad y estabilidad, no una optimización opcional.
