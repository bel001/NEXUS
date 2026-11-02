# TASK-003 â€” Policy Engine determinista (solo lectura)

Estado: IMPLEMENTACION AUTORIZADA, ACEPTACION FORMAL PENDIENTE.
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
