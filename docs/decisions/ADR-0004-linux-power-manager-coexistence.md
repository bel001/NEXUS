# ADR-0004: Coexistencia con gestores energéticos Linux

- Estado: Aceptada
- Fecha: 2026-10-30

## Contexto

TLP, auto-cpufreq y power-profiles-daemon pueden controlar tunables o perfiles que se solapan. Dos gestores pueden sobrescribir valores y producir comportamiento impredecible.

## Decisión

En la fase de detección, identificar gestores activos. En fases de escritura, elegir una sola autoridad para cada control. Preferir integrar una API documentada ya activa (por ejemplo D-Bus de power-profiles-daemon) antes que escribir directamente en sysfs.

Si TLP está activo, no competir por los mismos tunables. La aplicación podrá recomendar una configuración, pero no la aplicará mientras exista conflicto.

## Referencias

- https://linrunner.de/tlp/faq/conflicts.html
- https://upower.pages.freedesktop.org/power-profiles-daemon/gdbus-org.freedesktop.UPower.PowerProfiles.html
