# TASK-003 â€” Policy Engine determinista (solo lectura)

Estado: ACEPTADA FORMALMENTE (2026-11-02), tras smoke nativo Windows PASS.
Fecha: 2026-11-02. Base de implementacion: f823ba7 (TASK-002 cerrada con alcance acotado).

## Alcance de esta entrega

- Modulo puro `apps/agent/src/policy_engine.rs`, separado del acceso nativo.
- Entrada: `TelemetrySnapshot` V1 existente; sin alterar su esquema ni `HardwareInventory` V1.
- Salida JSON de recomendaciones explicables, tipadas por codigos estables, impacto cualitativo,
  riesgo y accion obligatoria `read_only`, con version de politica 1.
- CLI opt-in `adaptive-agent --recommendations` que reutiliza colectores ya aprobados
  CPU, memoria y alimentacion, una unica vez por invocacion. El modo sin argumentos
  y las tres banderas anteriores mantienen su comportamiento.
- Sin dependencia nueva, cambios al sistema, privilegios, historial, redes, procesos
  en primer plano, sensores termicos inventados, inferencia de energia ni acciones automaticas.

## Reglas conservadoras (heuristicas de diagnostico)

- CPU global >= 85% en una muestra valida: sugerir repetir medicion manualmente.
  No afirmar carga sostenida ni generar cambio de perfil.
- Memoria disponible <= 10% del total visible, con ambas lecturas disponibles y coherentes:
  sugerir nueva observacion. Calculo de fraccion con `u128` para evitar overflow.
- Si la fuente confirma bateria y el porcentaje es <= 20%: sugerir valorar conectar CA.
  No inferir estado de alimentacion ni porcentaje ausentes.
- Evidencias no disponibles, parciales o incoherentes: registrar codigos de ausencia,
  nunca reemplazar por cero ni emitir recomendaciones que requieran datos faltantes.

Los umbrales son heuristicas conservadoras de orientacion, NO presupuestos
contractuales, diagnosticos clinicos del equipo ni promesas de mejora o energia.
Se requieren multiples muestras y evaluacion futura para histeresis, deteccion
fisica de suspend/resume, estabilidad temporal, sensores termicos y acciones.

## Verificacion y aceptacion

- Tests de fixtures para umbrales, ausencias, ausencia de datos crudos y overflow.
- Validacion local: Rust 1.88.0 y stable, fmt, check, test y clippy -D warnings.
- CI: Windows + Ubuntu, 1.88 y stable, y smoke Linux heredado.
- La CI y pruebas deben verificarse por SHA tras publicacion. Una compilacion
  correcta NO significa aceptacion formal de TASK-003.
- Requiere decision posterior del propietario para aceptar esta entrega.
## Aceptacion formal TASK-003 (2026-11-02)

Decision del propietario: `ACEPTO TASK-003 CONDICIONADA A SMOKE NATIVO WINDOWS PASS. AUTORIZO DOCUMENTACION DE ACEPTACION, COMMIT Y PUSH.`

- Condicion comprobada localmente en Windows 11 fisico: `adaptive-agent --recommendations` compilado en perfil release con Rust 1.88.0 y `--locked` termina correctamente y devuelve JSON de politica version 1, `mode=read_only`, con colecciones tipadas y acciones exclusivamente `read_only`. El smoke no guarda ni muestra mediciones ni identificadores del equipo.
- Evidencia de implementacion previa: commit `7244aab`, CI Windows/Linux 4/4 y smoke Linux B4 2/2, comprobados antes de esta aceptacion.
- La aceptacion cubre evaluacion determinista bajo demanda, tres reglas conservadoras (CPU, memoria y bateria), manejo de evidencias ausentes y preservacion de TASK-001/002, sin dependencias nuevas ni cambios del sistema.
- No certifica carga sostenida, rendimiento universal, medicion energetica, deteccion fisica de suspension/reanudacion, pruebas de bateria Linux fisica ni mejoras logradas. Los umbrales son heuristicas orientativas, no limites contractuales.
- La CI del commit documental posterior a esta aceptacion se verificara por SHA: 4 jobs Windows/Linux y 2 pasos Linux smoke. La aceptacion no equivale a habilitar acciones, servicios residentes, telemetria continua ni una fase nueva.
