# Instrucciones para agentes de programación

## Contexto del producto

Construimos una plataforma local, multiplataforma y adaptativa para diagnóstico y optimización de PC/laptops. El nombre comercial está pendiente. Primera plataforma: Windows y Linux. La primera entrega funcional será read-only.

## Reglas no negociables

1. No implementar más allá de la tarea aprobada.
2. Antes de editar: inspeccionar el repositorio, resumir el plan y enumerar archivos que cambiarán.
3. No inventar resultados de pruebas. Ejecutar los comandos y reportar salida real.
4. No hacer cambios destructivos ni aplicar optimizaciones del sistema sin aprobación humana.
5. No ejecutar PowerShell, shell, scripts o comandos arbitrarios recibidos desde la UI o desde un modelo de IA.
6. No solicitar privilegios elevados para tareas de solo lectura si no son necesarios.
7. No desactivar seguridad, actualizaciones, firewall, servicios esenciales ni archivo de paginación.
8. No realizar overclock, undervolt, cambios de BIOS/UEFI ni control no documentado de ventiladores.
9. No vaciar la RAM ni eliminar cachés de forma agresiva.
10. No asumir que sensores o funciones existen. Representar explícitamente `unsupported`, `unavailable` o `permission_denied`.
11. No enviar telemetría a la nube. El producto debe funcionar offline.
12. No introducir dependencias sin justificar mantenimiento, licencia, tamaño, MSRV y superficie de ataque.
13. Mantener separados dominio, telemetría, políticas, acciones y adaptadores de plataforma.
14. Toda operación futura que modifique el sistema debe ser declarativa, validada, auditable y reversible.
15. En Linux, detectar gestores de energía existentes y no competir con ellos.

## Flujo obligatorio de trabajo

Para cada tarea:

1. Leer la tarea y documentación relacionada.
2. Presentar un plan breve antes de editar.
3. Implementar únicamente el alcance aprobado.
4. Añadir pruebas unitarias, de integración y fixtures donde corresponda.
5. Ejecutar `cargo fmt`, `cargo check`, `cargo test` y `cargo clippy`.
6. Informar archivos cambiados, pruebas ejecutadas, resultados, limitaciones y riesgos.
7. Esperar validación humana antes de iniciar la siguiente tarea.

## Criterios de código

- Rust Edition 2024.
- APIs pequeñas, tipadas y documentadas.
- Errores estructurados; no usar `unwrap()`/`expect()` en rutas operativas.
- `unsafe` solo en adaptadores de plataforma, aislado, documentado y revisado.
- No bloquear el hilo de UI.
- No crear loops de polling agresivos.
- Reutilizar colectores y actualizar solo métricas necesarias.
- Usar `cfg(target_os = "...")` para separar implementaciones específicas.
- Los datos desconocidos no se representarán como cero.

## Primera tarea autorizable

`TASK-001`: Hardware Discovery read-only. No implementar cambios de energía ni acciones de optimización en esa tarea.
