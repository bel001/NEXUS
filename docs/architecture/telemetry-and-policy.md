# Telemetría y motor de políticas

## Telemetría de bajo coste

- Mantener instancias de colectores cuando la biblioteca lo requiera; no recrearlas en cada tick.
- Actualizar solo los campos necesarios.
- Preferir notificaciones/eventos a polling.
- Usar intervalos adaptativos y backoff cuando el equipo está inactivo.
- Pausar muestreo durante suspensión y reanudar con un snapshot nuevo.
- No enumerar procesos completos si la pantalla que los necesita está cerrada.
- Separar telemetría rápida (estado básico) de diagnósticos profundos solicitados por el usuario.

La documentación de `sysinfo` recomienda reutilizar `System` y refrescar solo la información necesaria para reducir coste: https://docs.rs/sysinfo/latest/sysinfo/

## Snapshot

Cada valor debe incluir:
- valor tipado o ausencia;
- unidad;
- timestamp;
- fuente;
- estado: available, unsupported, unavailable, permission_denied o error;
- confianza/calidad cuando sea estimada.

Nunca representar un valor desconocido como cero.

## Policy Engine

Entrada:
- fuente de alimentación;
- porcentaje/estado de batería;
- carga CPU y memoria;
- proceso en primer plano (si el usuario autoriza esa observación);
- estado térmico cuando el sensor exista;
- capacidades del equipo;
- presencia de gestores externos.

Salida:
- recomendación o plan declarativo;
- razón explicable;
- impacto esperado cualitativo;
- riesgo;
- requisitos;
- acción reversible o `read_only`.

## Estabilidad de políticas

- Histéresis entre umbral de entrada y salida.
- Tiempo mínimo entre cambios.
- Cooldown después de una acción.
- Prioridad a la interacción en primer plano.
- No cambiar perfil por picos breves.
- Un estado de seguridad/thermal limit prevalece sobre una preferencia de rendimiento.

## IA futura

La primera versión usa reglas deterministas. La IA local podrá priorizar recomendaciones usando patrones locales, pero no tendrá permiso para generar ni ejecutar comandos. Toda propuesta de IA se convierte en una política tipada que pasa por validación normal.
