# TASK-002 - Revision de contrato y arquitectura

Estado: Fases A, B1, B2 y B3 aceptadas; TASK-002 ABIERTA.
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

## 15. Aceptacion formal de Fase A

Aceptacion expresa del propietario: 2026-11-01.

Evidencia:
- Implementacion: commit a66622c.
- CI: ejecucion 41385211033.
- Resultado CI: completed / success.
- Windows Rust 1.88: PASS.
- Windows Rust stable: PASS.
- Ubuntu Rust 1.88: PASS.
- Ubuntu Rust stable: PASS.
- Contrato TelemetrySnapshot y calculos puros.
- Pruebas, formato y Clippy verificados.
- Sin cambios al contrato HardwareInventory V1.

Decision:
FASE A ACEPTADA como incremento independiente.

Limitaciones:
- No incluye adquisicion nativa de telemetria.
- No incluye muestreo real ni servicio residente.
- No demuestra presupuestos de consumo del colector.
- No completa TASK-002.
- No autoriza Fase B.
- No autoriza nuevas dependencias ni cambios del sistema.

La Fase B requiere especificacion y autorizacion
expresas antes de comenzar su implementacion.

## 16. Autorizacion limitada de Fase B1

Fecha: 2026-11-01.
Estado: AUTORIZADA PARA IMPLEMENTACION, NO ACEPTADA.

Alcance:
- Captura real bajo demanda de memoria visible y disponible.
- Windows: WMI Win32_OperatingSystem, propiedades de lectura.
- Linux: /proc/meminfo, MemTotal y MemAvailable.
- Conversiones comprobadas y estados de disponibilidad.
- Timestamps, fuentes, privacidad y errores parciales.
- Pruebas deterministas y validacion nativa.
- CI Windows y Ubuntu, Rust 1.88 y stable.

Restricciones:
- Sin nuevas dependencias sin aprobacion separada.
- Sin procesos auxiliares ni conexiones de red.
- Sin privilegios elevados ni escritura de configuracion.
- Sin polling continuo ni historial persistente.
- Sin modificar HardwareInventory esquema 1.
- Sin alterar el contrato de Fase A innecesariamente.
- Si una fuente no es fiable, devolver unavailable o error.
- Medir coste real antes de afirmar limites cumplidos.

Fuera de alcance:
- Uso CPU real y ventanas de muestreo.
- Bateria, eventos energeticos y suspension.
- Servicios residentes, politicas y acciones.
- Cierre completo de TASK-002.

La aceptacion de B1 requiere pruebas y autorizacion
posterior. Este registro no autoriza commit ni push.

CI de partida: 41385864867.
Commit de partida: e17149d.

## 17. Aceptacion formal de Fase B1

Fecha: 2026-11-01.
Decision expresa del propietario: FASE B1 ACEPTADA.

Evidencia:
- Implementacion nativa: c755ddd.
- Prueba real Linux en CI: bf6b21f.
- CI: 41388343658, cuatro trabajos exitosos.
- Windows: lectura nativa de memoria verificada localmente.
- Linux: lectura de /proc/meminfo verificada en Ubuntu CI.
- Rust 1.88 y stable: PASS.
- Sin nuevas dependencias ni cambios del sistema.
- Captura privada original no versionada.

Limites:
- Validacion Linux realizada en runner Ubuntu; no en todos los equipos.
- Presupuesto de consumo aun no certificado.
- No se autorizan B2, CPU real ni bateria.
- Sin polling residente, procesos auxiliares ni telemetria de red.
- HardwareInventory V1 permanece sin cambios.
- TASK-002 sigue ABIERTA.

## 18. Autorizacion limitada de Fase B2 (CPU)

Autorizacion explicita del propietario en la conversacion: 2026-11-01.
Estado: AUTORIZADA PARA IMPLEMENTACION, NO ACEPTADA.

Alcance: dos muestras globales de CPU bajo demanda separadas por 250 ms;
Linux /proc/stat agregado; Windows WMI de contadores brutos _Total;
calculos checked, fuente, marca de tiempo, estados y pruebas deterministas;
prueba Linux en CI, validacion real Windows local; Rust 1.88 y stable.

Restricciones: sin unsafe, FFI, procesos auxiliares, red ni elevacion;
sin nuevas dependencias, sin polling continuo ni persistencia;
sin bateria, politicas ni cambios al inventario TASK-001;
sin alteracion de la funcion --telemetry-memory de B1.
No se certifican limites de energia, tiempo de WMI ni soporte global de
maquinas Windows con mas de 64 procesadores logicos. En ese caso no se
publica un porcentaje global potencialmente enganoso.

Este registro actualiza el estado B2 anterior, pero conserva la aceptacion B1.
Esta autorizacion NO aprueba ni publica B2 y NO permite commit o push.
La aceptacion de B2 se evaluara despues de las pruebas y de CI.
TASK-002 completa: ABIERTA.

## 19. Aceptacion formal de Fase B2

Fecha: 2026-11-02.
Decision expresa del propietario: FASE B2 ACEPTADA.

Evidencia:
- Implementacion: commit 7070b75.
- CI: ejecucion 41390907384, cuatro trabajos satisfactorios.
- Windows: medicion nativa de CPU validada localmente.
- Linux: lectura nativa de /proc/stat probada en Ubuntu CI.
- Rust 1.88 y stable: check, test, Clippy y formato aprobados.
- Medicion bajo demanda mediante dos muestras.
- Sin nuevas dependencias, unsafe, servicios residentes ni red.
- Sin modificar el contrato HardwareInventory V1.

Limitaciones:
- La prueba Linux acepta un rechazo seguro ante contadores invalidos.
- No se certifica disponibilidad universal de CPU en Linux.
- No se certifica cobertura global Windows con multiples grupos CPU.
- No se certifica el presupuesto de consumo energetico o latencia WMI.
- No se autorizan nuevas fases, bateria ni cambios del sistema.
- TASK-002 sigue ABIERTA.

## 20. Autorizacion limitada de Fase B3 (alimentacion y bateria)

Autorizacion expresa del propietario en la conversacion: 2026-11-02.
Estado: AUTORIZADA PARA IMPLEMENTACION, NO ACEPTADA.

Alcance:
- Lectura puntual del origen de alimentacion y porcentaje de bateria.
- Windows: WMI Win32_Battery, proyeccion solo de BatteryStatus y EstimatedChargeRemaining.
- Linux: /sys/class/power_supply, atributos tipo, online, present, status, capacity.
- Valores 0..100, evidencia de fuente, UTC, estados de no bateria y errores parciales.
- Pruebas deterministas y smoke test de sysfs nativo en Ubuntu CI.
- Rust 1.88/stable, Windows/Ubuntu. Sin cambio a HardwareInventory V1.

Limitaciones:
- Win32_Battery no demuestra CA en todos los estados; no inventar fuente.
- Sin bateria WMI no demuestra alimentacion CA.
- Multiples baterias no se combinan sin base de ponderacion.
- Una lectura no certifica suspend/resume ni presupuesto energetico.
- Sin unsafe, FFI, procesos auxiliares, red, elevacion, polling, escritura ni configuracion.
- Sin nuevas dependencias, nuevos servicios ni cambios de perfil energetico.
- No se autoriza commit, push ni la aceptacion automatica de B3.
- TASK-002 permanece ABIERTA; fases posteriores NO AUTORIZADAS.

## 21. Aceptacion formal de Fase B3

Fecha: 2026-11-02.
Decision expresa del propietario: FASE B3 ACEPTADA.

Evidencia:
- Implementacion: commit f473cd9.
- CI: ejecucion 41393237480, cuatro trabajos satisfactorios.
- Windows: lectura real de alimentacion y bateria disponible.
- Linux: proveedor sysfs probado en Ubuntu CI.
- Rust 1.88 y stable: validaciones satisfactorias.
- Sin modificar HardwareInventory V1.
- Sin nuevas dependencias ni cambios del sistema.

Limitaciones:
- El runner Linux no acredita una bateria fisica.
- Los estados ambiguos permanecen no disponibles.
- No se certifican todas las configuraciones de bateria.
- No se certifica consumo, latencia ni suspend/resume.
- No se autoriza polling permanente ni otras fases.
- TASK-002 permanece ABIERTA.
