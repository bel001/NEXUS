# TASK-002 - Revision de contrato y arquitectura

Estado: PROPUESTA PARA APROBACION, NO APROBADA.
Baseline: 66a37a7.
Documento principal: TASK-002-telemetry-read-only.md.

Esta revision consolida V0 y V1.
Hasta la aprobacion expresa, las decisiones son propuestas.
No modifica HardwareInventory ni autoriza codigo Rust.

## 1. Alcance propuesto

Primer incremento:
- Captura de telemetria local bajo demanda.
- Uso agregado de CPU.
- Memoria total visible y disponible.
- Fuente de alimentacion.
- Porcentaje de bateria si existe.
- Salida JSON tipada y versionada.

Fuera de alcance:
- Monitor residente y polling permanente.
- Historial persistente y telemetria de red.
- Procesos individuales.
- Policy Engine y Action Engine.
- Cambios energeticos o del sistema.

## 2. Contrato V1 propuesto

TelemetrySnapshot:
- schema_version: u32, inicial 1
- cpu_usage: CpuUsageReading
- memory_total_visible_bytes: TelemetryReading<u64>
- memory_available_bytes: TelemetryReading<u64>
- power_source: TelemetryReading<PowerSource>
- battery_percent: TelemetryReading<u8>
- issues: lista de incidencias sanitizadas

CpuUsageReading:
- usage_percent: TelemetryReading<f64>
- sample_window_ms: u64 o null
- cpu_scope: CpuScope

CpuScope:
- system_total
- processor_group
- unknown

PowerSource:
- ac
- battery

La propuesta V1 utiliza cpu_usage como objeto compuesto.
No duplica cpu_usage_percent en la raiz del snapshot.

## 3. TelemetryReading

Campos:
- value
- status
- source
- unit
- measured_at
- quality
- detail

Estados candidatos:
- available
- unavailable
- unsupported
- permission_denied
- error

Calidad:
- measured
- estimated

Reglas propuestas:
- available requiere value no nulo y valido.
- Los demas estados requieren value null.
- quality es null si no existe valor valido.
- Los porcentajes deben ser finitos y estar en 0..100.
- NaN e infinito no se serializan como datos validos.
- Desconocido nunca se convierte en cero.
- Cada lectura identifica su fuente nativa.
- Errores parciales no eliminan otras lecturas.
- detail e issues deben estar sanitizados.
- partial no se asigna automaticamente a escalares.

La politica definitiva para measured_at no disponible
requiere aprobacion expresa antes de implementar.

## 4. CPU Linux

Fuente candidata: /proc/stat, linea agregada cpu.

Se necesitan dos muestras de contadores acumulados.
No sumar guest ni guest_nice dos veces.

Convencion:
idle_bucket = idle + iowait
busy_delta = total_delta - idle_bucket_delta
usage = 100 * busy_delta / total_delta

Validar:
- Dos muestras del mismo alcance.
- Todos los deltas necesarios no negativos.
- Aritmetica sin desbordamiento.
- total_delta mayor que cero.
- Resultado finito dentro de 0..100.

iowait puede disminuir y tiene limitaciones conocidas.
Ante regresion, no publicar porcentaje para esa ventana.

## 5. CPU Windows

Fuente candidata: GetSystemTimes.

KernelTime incluye IdleTime.

total_delta = kernel_delta + user_delta
busy_delta = total_delta - idle_delta
usage = 100 * busy_delta / total_delta

Validar exito de llamadas, deltas y desbordamientos.

GetSystemTimes puede cubrir solamente un grupo
primario en equipos con mas de 64 procesadores.

No declarar system_total sin demostrar cobertura global.
Una lectura parcial no debe etiquetarse como global.

## 6. Ventana CPU

sample_window_ms es la duracion REAL medida usando
reloj monotono, no la duracion solicitada.

- Primera lectura: linea base, sin porcentaje.
- Dos lecturas coherentes: porcentaje candidato.
- Delta total cero: lectura no disponible.
- Cambio de alcance: invalidar ventana.
- Regresion: invalidar ventana.
- Suspension detectada: invalidar ventana.

Para una medicion disponible, sample_window_ms debe
ser mayor que cero y el alcance debe estar justificado.

La duracion exacta y el timeout se definiran mediante
pruebas de coste y estabilidad, antes de implementar.

## 7. Memoria

Windows:
- GlobalMemoryStatusEx.
- ullTotalPhys.
- ullAvailPhys.

Linux:
- /proc/meminfo.
- MemTotal.
- MemAvailable.

Usar bytes tipados.
Verificar total mayor que cero cuando se derive uso.
No calcular porcentajes con denominadores invalidos.

MemAvailable de Linux es una estimacion del kernel.
No afirmar equivalencia semantica perfecta con Windows.

## 8. Alimentacion y bateria

Windows:
- GetSystemPowerStatus.

Linux:
- /sys/class/power_supply.

PowerSource representa la fuente de alimentacion.
No representa la presencia fisica de bateria.

Un equipo puede utilizar CA y tener bateria presente.

Si se demuestra ausencia de bateria, el porcentaje
puede quedar unsupported.

Si falta evidencia, utilizar unavailable.
Si hay error o falta de permisos, conservar la causa.

En Windows, un porcentaje centinela 255 significa
desconocido; nunca debe publicarse como 255 por ciento.

## 9. Timestamps y suspension

Cada lectura tiene measured_at propio.

El reloj monotono sirve para intervalos CPU.
La fecha UTC no se usa para calcular los deltas CPU.

Ante suspension detectada o discontinuidad,
la ventana CPU deja de ser valida.

No afirmar deteccion infalible de suspension.

La politica de timestamp UTC no disponible y la
estrategia de reanudacion requieren aprobacion.

## 10. Arquitectura Rust

Workspace actual: cinco paquetes.

Grafo existente:
- agent depende de domain y hardware-discovery.
- agent selecciona adaptador segun plataforma.
- platform-linux y platform-windows dependen de
  domain y hardware-discovery.
- hardware-discovery depende de domain.
- domain no depende de los otros crates.

Opcion propuesta para TASK-002:
- Mantener tipos compartidos independientes del SO.
- Separar telemetria de hardware-discovery.
- Evaluar un nuevo crate telemetry que dependa de domain.
- Los adaptadores de plataforma podrian depender de telemetry.
- Agent seleccionaria el proveedor de su plataforma.
- No introducir dependencias ciclicas.

La necesidad del nuevo crate aun requiere aprobacion.
No agregar dependencias ni modificar Cargo.toml ahora.

## 11. Seguridad y presupuesto

Condiciones obligatorias:
- Read-only.
- Sin privilegios elevados por defecto.
- Sin ejecutar herramientas auxiliares de recopilacion.
- Sin telemetria de red.
- Sin seriales, rutas privadas o usuarios.
- Sin modificar energia, servicios o controladores.
- Sin almacenamiento persistente por defecto.
- Sin afectar el esquema 1 de HardwareInventory.

Antes de implementar:
- Evaluar ventana y timeout CPU.
- Definir cancelacion.
- Establecer presupuesto de latencia.
- Establecer presupuesto de CPU del colector.
- Establecer presupuesto de memoria.
- Definir metodologia reproducible de medicion.

Ningun presupuesto se declara cumplido sin pruebas.

## 12. Pruebas necesarias

- Fixtures deterministas para CPU.
- Primera lectura sin porcentaje.
- Deltas cero, negativos e inconsistentes.
- Aritmetica comprobada y overflow.
- Limitaciones multiprocesador Windows.
- Memoria disponible, ausente e inconsistente.
- Equipos con bateria y sin bateria.
- Alimentacion conocida y desconocida.
- Porcentajes fuera de rango.
- Timestamps y suspension.
- Errores parciales.
- Contrato JSON versionado.
- Revision de privacidad.
- Medicion real del coste.
- Pruebas nativas Windows y Linux.
- Rust 1.88 y stable.
- CI Windows y Ubuntu.

## 13. Decisiones pendientes de aprobacion

- [ ] Aprobar contrato V1.
- [ ] Aprobar enums, unidades y estados.
- [ ] Aprobar fuentes nativas.
- [ ] Aprobar intervalo y timeout CPU.
- [ ] Aprobar politica de timestamps.
- [ ] Aprobar alcance multiprocesador.
- [ ] Aprobar manejo de suspension.
- [ ] Decidir nuevo crate telemetry.
- [ ] Revisar dependencias y licencias.
- [ ] Aprobar presupuesto y pruebas.
- [ ] Autorizar expresamente implementacion.

Estado general: TASK-002 NO APROBADA EN SU CONJUNTO.
Fase A: autorizada expresamente por el propietario el 2026-11-01.
Fase B y aceptacion final: pendientes.

Fuentes:
https://doc.rust-lang.org/cargo/commands/cargo-metadata.html
https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getsystemtimes
https://www.kernel.org/doc/html/latest/filesystems/proc.html
https://docs.kernel.org/power/power_supply_class.html

## 14. Autorizacion limitada de Fase A

La autorizacion expresada en consola permite unicamente
el primer incremento de codigo, bajo estas condiciones:

- Modificar adaptive-domain para agregar un contrato
  de telemetria independiente de HardwareInventory.
- Definir TelemetrySnapshot, TelemetryReading<T>,
  CpuUsageReading, CpuScope y PowerSource.
- Usar serde y time ya existentes en adaptive-domain.
- Estados: available, unavailable, unsupported,
  permission_denied y error.
- available requiere un valor valido y timestamp UTC.
- Estados sin valor requieren value null.
- Rechazar porcentajes no finitos o fuera de 0..100.
- quality solo existe cuando existe valor valido.
- CPU disponible exige scope system_total y
  sample_window_ms mayor que cero.
- Funciones matematicas puras con aritmetica comprobada.
- Fixtures y pruebas para errores y valores extremos.
- Mantener #![forbid(unsafe_code)].
- Mantener sin cambios HardwareInventory esquema 1.

No se autorizan todavia:

- APIs nativas Windows o Linux;
- muestreo temporal real;
- nuevos crates o dependencias;
- tareas residentes;
- cambios del sistema;
- motor de politicas o acciones.

La fase B requiere una aprobacion separada que defina
ventanas de muestreo, suspension, timeouts y presupuesto
de consumo.

Esta autorizacion no significa que TASK-002 este
completada ni que sus criterios finales esten aprobados.
