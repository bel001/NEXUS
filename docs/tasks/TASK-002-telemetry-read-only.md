# TASK-002 — Telemetria local read-only

- Estado: TASK-002 CIERRE ACOTADO ACEPTADO (2026-11-02); CI del commit de cierre sujeta a verificacion pospush.
- Prioridad propuesta: P0
- Plataforma objetivo: Windows 11 y Linux x86_64
- Dependencia: TASK-001 aceptada
- Nivel de accion: L0, solo lectura

## 1. Objetivo propuesto

Definir un sistema de observacion local de bajo consumo
que mida el estado cambiante del equipo sin modificarlo.

Hardware Discovery identifica los componentes.
Telemetry observaria su estado durante el funcionamiento.

Este documento es una propuesta de alcance. No autoriza
implementacion ni modifica decisiones de arquitectura.

## 2. Capacidades candidatas

- Uso de CPU, con unidad y metodo de calculo documentados.
- Estado de memoria: total visible, disponible y uso estimado.
- Alimentacion: CA/bateria cuando exista fuente aprobada.
- Estado de bateria cuando la fuente permita demostrarlo.
- Snapshot local tipado y versionado.
- Estados explicitos para informacion no disponible.
- Observaciones reproducibles mediante fixtures.

La inclusion definitiva de cada metrica requiere verificar
fuentes nativas, permisos, precision, coste y portabilidad.

## 3. Contrato a definir

La telemetria necesita su propio contrato, distinto del
HardwareInventory de TASK-001.

Cada medicion deberia identificar:
- valor tipado o ausencia;
- unidad;
- instante de medicion;
- fuente;
- estado de disponibilidad;
- calidad/confianza cuando corresponda.

No reutilizar incorrectamente captured_at como timestamp
individual ni alterar el esquema 1 de HardwareInventory.

No convertir datos desconocidos en cero.

## 4. Restricciones obligatorias

- Modo read-only.
- Sin privilegios elevados en uso normal.
- Sin modificar energia, rendimiento, servicios o drivers.
- Sin ejecutar comandos externos para recopilar datos.
- Sin conexiones remotas ni telemetria de red.
- Sin nombres de usuario, procesos personales o seriales.
- Sin persistencia de historial por defecto.
- Sin actividad oculta de monitorizacion permanente.
- Respetar suspensiones, errores parciales y disponibilidad.
- Consumo de CPU y memoria del colector debe medirse.
- Sin agregar dependencias antes de justificar y aprobarlas.

## 5. Fuera de alcance

- Motor de politicas.
- Aplicacion de optimizaciones.
- Action Engine y Rollback.
- Tauri, React y TypeScript.
- SQLite.
- Inteligencia artificial.
- Benchmark de rendimiento.
- Modificacion de perfiles energeticos.
- Telemetria remota.

## 6. Decisiones abiertas

- Lista exacta de metricas del primer incremento.
- Fuente nativa aprobada para cada sistema operativo.
- Semantica y calculo de porcentaje de uso de CPU.
- Formato y version del snapshot.
- Muestreo puntual frente a muestreo periodico.
- Intervalos, limites, backoff y suspension.
- Reutilizacion de tipos del dominio sin romper TASK-001.
- Necesidad de nuevos crates y dependencias.
- Politica de privacidad de las muestras.
- Limites verificables de coste y latencia.

Estas decisiones requieren revision de arquitectura
antes de iniciar codigo.

## 7. Criterios candidatos de aceptacion

- [ ] Diseno tecnico aprobado por el propietario.
- [ ] Fuentes de lectura Windows y Linux documentadas.
- [ ] Contrato tipado y versionado definido.
- [ ] Ausencias y fallos parciales representados.
- [ ] Pruebas deterministas con fixtures.
- [ ] Sin modificacion de la configuracion del equipo.
- [ ] Sin privilegios innecesarios ni telemetria de red.
- [ ] Consumo del colector medido y documentado.
- [ ] Rust MSRV y stable verificados.
- [ ] CI Windows y Ubuntu satisfactoria.
- [ ] Evidencia nativa separada de pruebas simuladas.
- [ ] Documentacion y limitaciones actualizadas.
- [ ] Aceptacion final expresa del propietario.

## 8. Referencias existentes

- docs/architecture/overview.md
- docs/architecture/modules.md
- docs/architecture/telemetry-and-policy.md
- docs/tasks/definition-of-done.md
- docs/continuity/NEXUS-HANDOFF-POST-TASK001.md

## 9. Estado de implementacion

Fase A: IMPLEMENTADA Y ACEPTADA el 2026-11-01.
Commit de implementacion: a66622c.
CI: 41385211033, cuatro trabajos satisfactorios.
Fase B1: IMPLEMENTADA Y ACEPTADA el 2026-11-01 (c755ddd y bf6b21f; CI 41388343658). Fase B2: IMPLEMENTADA Y ACEPTADA el 2026-11-02 (7070b75; CI 41390907384).
TASK-002 completa: ABIERTA.

Fase B3: IMPLEMENTADA Y ACEPTADA el 2026-11-02 (f473cd9; CI 41393237480).

Las autorizaciones y aceptaciones de A, B1, B2 y B3
constan en TASK-002-design-review.md.

Este documento no autoriza nuevas fases, dependencias,
servicios residentes ni cambios del sistema.

Fecha de creacion: 2026-11-01
Baseline: 66a37a7

## 10. Investigacion de fuentes nativas

Estado: INVESTIGACION DOCUMENTAL, NO APROBADA.

Las fuentes siguientes son candidatas para TASK-002.
Todavia no han sido integradas en NEXUS.

### 10.1 CPU Linux

Fuente: `/proc/stat`, linea agregada `cpu`.

- Los contadores son acumulados.
- Se necesitan dos muestras para calcular un porcentaje.
- La primera muestra solo establece la linea base.
- Validar diferencias y denominador mayor que cero.
- Evitar sumar `guest` y `guest_nice` dos veces.
- `iowait` tiene limitaciones de interpretacion.
- Si los contadores retroceden, devolver unavailable.
- El intervalo debe medirse con un reloj monotono.

Fuente oficial:
https://www.kernel.org/doc/html/latest/filesystems/proc.html

### 10.2 CPU Windows

Fuente: `GetSystemTimes`.

Contadores: IdleTime, KernelTime y UserTime.

- KernelTime incluye tiempo inactivo.
- Calcular diferencias entre dos lecturas.
- Total = diferencia KernelTime + diferencia UserTime.
- Ocupado = Total - diferencia IdleTime.
- Validar que las diferencias sean consistentes.
- No inferir carga instantanea de una sola lectura.
- Considerar los limites de grupos de procesadores
  cuando existen mas de 64 procesadores logicos.

Fuente oficial:
https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getsystemtimes

### 10.3 Memoria Linux

Fuente: `/proc/meminfo`.

- MemTotal: memoria RAM utilizable visible.
- MemAvailable: estimacion de memoria disponible.
- No sustituir MemAvailable por MemFree.
- Validar las unidades y convertir a bytes.
- Si faltan datos, representar unavailable.
- No confundir RAM visible con RAM instalada.

Fuente:
https://man7.org/linux/man-pages/man5/proc_meminfo.5.html

### 10.4 Memoria Windows

Fuente: `GlobalMemoryStatusEx`.

Campos candidatos:
- ullTotalPhys
- ullAvailPhys
- dwMemoryLoad

- Verificar el resultado de la llamada.
- Respetar las unidades nativas.
- La lectura puede variar entre llamadas.
- No asumir equivalencia exacta con Linux.
- No confundir memoria visible con instalada.

Fuente:
https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-globalmemorystatusex

### 10.5 Alimentacion Linux

Fuente: `/sys/class/power_supply`.

Propiedades candidatas:
- type
- online
- status
- capacity

- Distinguir fuentes de alimentacion y baterias.
- No asumir que todos los controladores tienen
  los mismos atributos.
- Una bateria ausente no equivale a un error.
- No escribir en sysfs.
- No inventar porcentajes o estados desconocidos.

Fuente:
https://docs.kernel.org/power/power_supply_class.html

### 10.6 Alimentacion Windows

Fuente: `GetSystemPowerStatus`.

Estructura: SYSTEM_POWER_STATUS.

Campos candidatos:
- ACLineStatus
- BatteryFlag
- BatteryLifePercent

- El valor 255 puede significar desconocido.
- Nunca tratar 255 como porcentaje.
- Distinguir ausencia de bateria de fallo.
- No inferir descarga solamente por bateria baja.
- Comprobar errores de la llamada.

Fuentes:
https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getsystempowerstatus
https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-system_power_status

## 11. Contrato propuesto

Definir TelemetrySnapshot separado de HardwareInventory.

Propuestas:

- Esquema de telemetria versionado.
- Valores tipados y unidades explicitas.
- Timestamps asociados a mediciones.
- Reloj monotono para intervalos de CPU.
- Fuente y disponibilidad por observacion.
- Errores parciales sin perder otras metricas.
- Sin identificadores personales o procesos.
- Sin alterar HardwareInventory esquema 1.
- Sin convertir valores desconocidos en cero.

El contrato concreto necesita revision y aprobacion.

## 12. Primer incremento recomendado

- Captura bajo demanda, no servicio permanente.
- Memoria visible y disponible.
- Alimentacion CA/bateria cuando exista.
- Porcentaje de CPU con dos lecturas validas.
- Salida JSON tipada y versionada.
- Sin almacenamiento de historial por defecto.
- Sin red ni procesos auxiliares.
- Sin modificar energia o rendimiento del equipo.

El intervalo de CPU y el presupuesto de recursos
siguen pendientes de una decision justificada.

## 13. Estrategia de pruebas

### Contrato

- Serializacion y version.
- Valores ausentes.
- Unidades y timestamps.
- Errores parciales.

### CPU

- Primera lectura sin porcentaje.
- Diferencias correctas.
- Denominador cero.
- Contadores decrecientes.
- Desbordamientos.
- Limitaciones de procesadores Windows.

### Memoria

- Lecturas disponibles y ausentes.
- Conversiones a bytes.
- Total cero o inconsistente.
- Diferencias semanticas entre plataformas.

### Alimentacion

- Equipo con bateria y sin bateria.
- Fuente CA disponible.
- Estados desconocidos.
- Porcentaje fuera de rango.
- Errores del proveedor.

### Seguridad y coste

- Sin escritura sobre el sistema.
- Sin ejecucion de comandos auxiliares.
- Sin red ni privilegios elevados.
- Medicion de CPU, RAM y latencia del colector.
- Rust 1.88 y stable.
- CI Windows y Ubuntu.
- Pruebas reales separadas de fixtures.

## 14. Decisiones pendientes

- [ ] Aprobar las metricas iniciales.
- [ ] Aprobar las fuentes nativas.
- [ ] Definir el calculo CPU detallado.
- [ ] Aprobar el contrato TelemetrySnapshot.
- [ ] Definir timestamps y reloj monotono.
- [ ] Definir el intervalo de muestreo.
- [ ] Definir limites de CPU y memoria del colector.
- [ ] Resolver suspension y reanudacion.
- [ ] Revisar dependencias y licencias.
- [ ] Decidir nuevos crates o reutilizacion.
- [ ] Aprobar el diseno antes de implementar.

## 15. Estado de esta investigacion

La investigacion tecnica esta documentada,
pero las fuentes nativas no han sido ejecutadas
por TASK-002.

TASK-002 permanece como BORRADOR NO APROBADO.

No se autorizan implementaciones, cambios de
dependencias, nuevos servicios ni optimizaciones.

## 16. Contrato TelemetrySnapshot V0

Estado: PROPUESTA DE ARQUITECTURA, NO APROBADA.

Se propone un contrato separado del HardwareInventory
existente. No se modifica el esquema de TASK-001.

### 16.1 TelemetrySnapshot

Campos candidatos:

- schema_version: entero positivo, inicial 1.
- cpu_usage_percent: TelemetryReading<f64>.
- memory_total_visible_bytes: TelemetryReading<u64>.
- memory_available_bytes: TelemetryReading<u64>.
- power_source: TelemetryReading<PowerSource>.
- battery_percent: TelemetryReading<u8>.
- issues: lista de incidencias sanitizadas.

PowerSource propuesto:

- ac
- battery

Un origen desconocido se expresa mediante ausencia
de valor y estado apropiado, no mediante una suposicion.

### 16.2 TelemetryReading<T>

Campos candidatos:

- value: T o null.
- status: estado tipado de disponibilidad.
- source: origen tipado de medicion.
- unit: unidad tipada cuando corresponda.
- measured_at: instante UTC RFC3339 o null.
- quality: measured o estimated, si corresponde.
- detail: diagnostico opcional sanitizado.

Los estados deben respetar la semantica del dominio.
Los valores desconocidos no se convierten en cero.

La inclusion de partial para escalares, el comportamiento
sin reloj UTC fiable y los nombres finales de campos
requieren una decision especifica.

No debe añadirse timestamp a Observation<T> de
HardwareInventory. TelemetryReading es otro contrato.

### 16.3 Invariantes propuestas

- schema_version debe ser valido.
- value null no equivale a valor cero.
- Porcentajes disponibles deben estar entre 0 y 100.
- Bytes y contadores nunca seran negativos.
- Fallos parciales no anulan otras lecturas.
- Cada lectura conserva su fuente.
- Un valor disponible requiere evidencia suficiente.
- Ningun campo contiene seriales, MAC, usuarios o rutas.
- Los detalles de error se sanitizan.
- El JSON de telemetria no modifica TASK-001.

### 16.4 Tiempos

Cada lectura tiene su instante de observacion.

Para uso CPU se necesitan dos muestras de contadores.
El intervalo entre ellas debe medirse mediante
reloj monotono. Ese intervalo no es un timestamp UTC.

La lectura de uso CPU se asocia al final de la
ventana medida y debe indicar el intervalo utilizado.

Una suspension o discontinuidad detectada invalida
el porcentaje en vez de producir un dato engañoso.

El tratamiento exacto de relojes no disponibles y
ventanas de muestreo sigue pendiente de aprobacion.

## 17. Reglas matematicas de CPU V0

### 17.1 Linux

Fuente candidata: primera linea cpu de /proc/stat.

Contadores para el total:

user + nice + system + idle + iowait +
irq + softirq + steal.

No sumar guest ni guest_nice otra vez.

Convencion propuesta para ocupacion:

idle_bucket = idle + iowait
busy_delta = total_delta - idle_bucket_delta
usage_percent = 100 * busy_delta / total_delta

Esta formula considera iowait no ocupado a efectos
de porcentaje, aunque iowait no representa una medida
fiable del tiempo ocioso de una CPU.

Antes del calculo:

- Confirmar dos muestras validas del mismo alcance.
- Validar contadores mediante aritmetica comprobada.
- Invalidar lecturas ante regresiones.
- Invalidar total_delta igual a cero.
- Invalidar resultados fuera del rango 0..100.
- Documentar los efectos de cambios de topologia.

### 17.2 Windows

Fuente candidata: GetSystemTimes.

Para dos muestras:

total_delta = kernel_delta + user_delta
busy_delta = total_delta - idle_delta
usage_percent = 100 * busy_delta / total_delta

KernelTime ya incluye IdleTime.

Validar:

- Exito de ambas llamadas.
- Contadores monotonicamente consistentes.
- kernel_delta >= idle_delta.
- total_delta > 0.
- Operaciones aritmeticas sin desbordamiento.
- Resultado entre 0 y 100.

Cuando exista mas de un grupo de procesadores,
GetSystemTimes puede no cubrir todo el equipo.
No llamar global a una lectura de alcance parcial.

### 17.3 Resultado de primera muestra

No generar porcentaje tras una sola lectura.
La primera muestra establece la linea base.

Si el segundo muestreo falla, representar
un estado no disponible con detalle sanitizado.

## 18. Integracion y coste propuestos

La primera version seria bajo demanda:

- Una peticion produce un snapshot.
- Sin bucle oculto permanente.
- Sin historial persistente.
- Sin recopilacion de procesos individuales.
- Sin conexiones de red.
- Sin modificar configuraciones del sistema.
- Sin incorporar un Policy Engine ni Action Engine.

Las fuentes nativas pertenecen a adaptadores.
Los tipos compartidos no dependen de APIs del SO.
La orquestacion no ejecuta comandos auxiliares.

Se debe decidir antes de implementar:

- Ubicacion final de los tipos de telemetria.
- Necesidad o no de un crate telemetry.
- Dependencias nativas y sus licencias.
- Duracion de la ventana CPU.
- Timeout y cancelacion de la captura.
- Manejo de suspension y reanudacion.
- Presupuesto maximo de latencia.
- Presupuesto maximo de CPU del colector.
- Presupuesto maximo de memoria del colector.

No establecer limites numericos sin una
medicion de referencia y aprobacion documentada.

## 19. Puerta de aprobacion de TASK-002

Para aprobar el diseno deberan resolverse:

- [ ] Contrato exacto y esquema JSON.
- [ ] Fuentes nativas por plataforma.
- [ ] Modelo de estados, unidades y calidad.
- [ ] Regla de timestamps y reloj monotono.
- [ ] Interpretacion de iowait.
- [ ] Alcance de CPU en sistemas multiprocesador.
- [ ] Intervalos, timeouts y suspension.
- [ ] Limites de consumo verificables.
- [ ] Plan de pruebas y fixtures.
- [ ] Dependencias y licencias.
- [ ] Grafo de crates sin ciclos.
- [ ] Privacidad de muestras y errores.
- [ ] Aprobacion expresa del propietario.

Estado final de este documento:
BORRADOR DE DISENO, NO APROBADO.

No se autoriza todavia la implementacion de TASK-002.

## 20. Revision de arquitectura V1

Estado: PROPUESTA V1, PENDIENTE DE APROBACION.

Esta revision complementa las secciones V0 anteriores.
En caso de contradiccion, los siguientes puntos son
propuestas de correccion, no decisiones aprobadas.

### 20.1 Alcance del primer incremento

Se propone una captura local bajo demanda con:

- Uso agregado de CPU, si hay dos muestras validas.
- Memoria total visible y memoria disponible.
- Estado de alimentacion CA/bateria, si es demostrable.
- Porcentaje de bateria, si existe fuente valida.
- Lecturas independientes con estados explicitos.
- Un JSON de telemetria separado de HardwareInventory.

No incluye un monitor residente, servicios nuevos,
historial persistente, procesos individuales,
optimizaciones ni un motor de politicas.

### 20.2 Estructura de datos V1 candidata

TelemetrySnapshot:

- schema_version: u32, inicialmente 1.
- cpu_usage: CpuUsageReading.
- memory_total_visible_bytes: TelemetryReading<u64>.
- memory_available_bytes: TelemetryReading<u64>.
- power_source: TelemetryReading<PowerSource>.
- battery_percent: TelemetryReading<u8>.
- issues: lista de errores sanitizados.

CpuUsageReading:

- usage_percent: TelemetryReading<f64>.
- sample_window_ms: u64 o null.
- cpu_scope: CpuScope.

CpuScope propuesto:

- system_total
- processor_group
- unknown

sample_window_ms describe la ventana REAL medida con
reloj monotono, no el tiempo solicitado para muestrear.

La medicion solo puede declararse global cuando la
fuente demuestra que cubre los procesadores relevantes.
No inventar identificadores de grupos.

### 20.3 Estados y calidad

TelemetryReading<T> conserva:

- value
- status
- source
- unit
- measured_at
- quality
- detail

Estados candidatos para lecturas escalares:

- available
- unavailable
- unsupported
- permission_denied
- error

La semantica `partial` queda pendiente de decidir
para estructuras compuestas, no se asigna
automaticamente a un escalar.

Un status available necesita un valor valido.
Un valor desconocido no se representa mediante cero.

Calidad candidata:

- measured
- estimated

No confundir confianza de una estimacion con estado
de disponibilidad.

### 20.4 Alimentacion y bateria

PowerSource:

- ac
- battery

Si no se puede determinar cual alimenta al equipo,
la lectura queda sin valor y con estado adecuado.

Un equipo puede tener bateria y utilizar CA
simultaneamente. La fuente activa no describe
por si sola la presencia de una bateria.

Si se demuestra que no existe bateria, su porcentaje
puede representarse como unsupported.

Si la fuente falla o falta evidencia suficiente,
utilizar unavailable, permission_denied o error
segun la causa, sin afirmar ausencia.

### 20.5 Tiempo y suspension

Las lecturas de telemetria tienen su propio
measured_at, distinto del inventario TASK-001.

Para porcentajes CPU se usa reloj monotono
y dos muestras del mismo alcance.

No generar porcentaje tras una sola muestra.

Si se detecta suspension, reinicio de contadores,
cambio incompatible de alcance o discontinuidad
no interpretable, invalidar el porcentaje.

Si no hay un mecanismo fiable para detectar
suspension, documentar la limitacion.
No afirmar deteccion garantizada.

La politica ante falta de reloj UTC fiable
debe definirse antes de implementar.

### 20.6 Seguridad

- Read-only sin privilegios elevados por defecto.
- Sin llamadas de red.
- Sin procesos auxiliares de recopilacion.
- Sin escritura a sysfs o configuraciones Windows.
- Sin nombres de usuario, rutas privadas o seriales.
- Diagnosticos sanitizados.
- Sin polling oculto o permanente.
- Sin cambios al contrato HardwareInventory V1.

## 21. Grafo Cargo actual

Grafo derivado de `cargo metadata --no-deps`,
con las dependencias declaradas entre paquetes
del workspace.

Las dependencias condicionadas por plataforma
no significan que ambos adaptadores se ejecuten
simultaneamente.
- `adaptive-agent` -> `adaptive-domain`
- `adaptive-agent` -> `adaptive-hardware-discovery`
- `adaptive-agent` -> `adaptive-platform-linux`
- `adaptive-agent` -> `adaptive-platform-windows`
- `adaptive-hardware-discovery` -> `adaptive-domain`
- `adaptive-platform-linux` -> `adaptive-domain`
- `adaptive-platform-linux` -> `adaptive-hardware-discovery`
- `adaptive-platform-windows` -> `adaptive-domain`
- `adaptive-platform-windows` -> `adaptive-hardware-discovery`

## 22. Integracion propuesta

El modulo conceptual telemetry no constituye
todavia un crate confirmado del workspace.

Recomendacion de arquitectura:

- Mantener tipos compartidos libres de APIs nativas.
- Mantener las llamadas Windows/Linux en adaptadores.
- Separar adquisicion, normalizacion y serializacion.
- El agente selecciona el adaptador del target.
- El proveedor no debe decidir ni aplicar politicas.
- Evitar dependencias ciclicas.

No se crea un crate nuevo durante esta fase.
La decision de agregar un crate de telemetria
queda reservada a la aprobacion del diseno.

## 23. Pruebas y presupuesto

Pruebas candidatas obligatorias:

- CPU: dos muestras, regresiones y delta cero.
- CPU: overflow y alcance incompleto.
- Linux: semantica documentada de iowait.
- Windows: grupos de mas de 64 procesadores.
- Memoria: conversiones y valores inconsistentes.
- Alimentacion: CA, bateria, ausencia y desconocidos.
- Suspension: invalidacion de ventanas no fiables.
- Serializacion, timestamps, fuentes y estados.
- Errores parciales sin perder otras lecturas.
- Pruebas nativas separadas de fixtures.
- CI en Windows/Ubuntu con Rust 1.88 y stable.

Antes de implementar se deben aprobar:

- Intervalo y timeout de CPU.
- Politica de cancelacion.
- Limite de latencia del snapshot.
- Coste maximo de CPU del colector.
- Consumo maximo de memoria del colector.
- Metodologia de medicion de referencia.

No fijar presupuestos arbitrarios sin evidencia.

## 24. Resultado de la revision V1

Hallazgos corregidos documentalmente:

- El contrato ahora propone sample_window_ms.
- El contrato ahora propone cpu_scope.
- Se distingue fuente de alimentacion y presencia
  de bateria.
- Se documentan condiciones de invalidacion CPU.
- Se incorpora el grafo Cargo local observado.

Pendiente de aprobacion:

- Semantica exacta del JSON y los enums.
- Tiempo UTC ausente y estados parciales.
- Intervalos, presupuestos y suspension.
- Nuevo crate frente a reutilizacion de modulos.
- Dependencias/licencias que pudieran necesitarse.
- Revision arquitectonica y consentimiento del dueno.

Esta revision es un BORRADOR V1.
No es aprobacion de TASK-002.

No se modifica el codigo Rust.
No se autoriza implementar ni publicar este diseno.

Referencias:
https://doc.rust-lang.org/cargo/commands/cargo-metadata.html
https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getsystemtimes
https://www.kernel.org/doc/html/latest/filesystems/proc.html
https://docs.kernel.org/power/power_supply_class.html

## 25. Fase C - auditoria de criterios de cierre (2026-11-02)

Estado: REVISION DE CIERRE REALIZADA; TASK-002 ABIERTA. Esta seccion prevalece como resumen de estado sobre las propuestas historicas de las secciones anteriores, que se conservan como registro de diseno y no se reescriben retroactivamente. La autorizacion conjunta recibida permite la auditoria C, sus comprobaciones, commit y push; no autoriza el cierre final de TASK-002.

### Evidencia consolidada hasta la base 8ff0e52

- Fase A: contrato tipado de telemetria aceptado (`e17149d`).
- B1: memoria Windows/Linux aceptada (`6565d34`).
- B2: CPU bajo demanda Windows/Linux aceptada (`144a6be`).
- B3: alimentacion y bateria aceptada (`163b968`).
- B4: medicion de coste del colector aceptada y publicada (`8ff0e52`). La ejecucion `Rust CI #19` (`41498563272`) verifico cuatro trabajos y los dos smoke Linux.
- Se identifican pruebas deterministas ya presentes en `crates/domain/src/telemetry.rs`: `cpu_linux_two_samples_and_iowait`, `cpu_linux_detects_overflow`, `wmi_inverse_timer_checked_deltas` y `cpu_window_requires_global_scope_and_positive_duration`. Comprueban errores de muestras, contadores y ventanas, pero NO detectan por si solas una suspension fisica.

### Matriz de los 13 criterios candidatos (seccion 7)

1. Diseno tecnico: evidencia de aceptacion por fases; aceptacion integral de TASK-002 PENDIENTE.
2. Fuentes nativas Windows/Linux: documentadas y comprobadas en los alcances B1-B3.
3. Contrato tipado/versionado: implementado y aceptado en A.
4. Fallos y ausencias: representados mediante estados tipados, con pruebas.
5. Fixtures y pruebas deterministas: evidencia existente; no duplicar pruebas sin identificar un caso faltante.
6. Sin cambios del sistema: restriccion observada dentro del alcance implementado, no auditoria exhaustiva del entorno.
7. Sin elevacion ni telemetria de red: restriccion de producto; las herramientas externas de desarrollo/CI son distintas del colector.
8. Coste del colector: medido en Windows; Linux smoke funcional. No se mide energia.
9. MSRV y stable: CI 4/4 del commit B4 documental.
10. CI Windows/Ubuntu: PASS 4/4 en ejecucion 41498563272.
11. Evidencia nativa vs fixtures: diferenciada; runner Linux no representa un portatil con bateria real.
12. Documentacion y limites: reconciliados por esta fase C, sujetos a CI del nuevo commit.
13. Aceptacion final de TASK-002: PENDIENTE de decision expresa del propietario.

### Limitaciones y decisiones pendientes para cierre

- Suspension/reanudacion: las pruebas de regresion, delta cero y ventana invalida cubren discontinuidades detectables; no existe evidencia suficiente para certificar deteccion universal de suspension/reanudacion ni una prueba fisica despues de resume. No afirmar garantia.
- Rendimiento: p50/p95 de Windows provienen de 10 muestras por modo, con p95 nearest-rank igual al maximo observado. No fijar limites contractuales universales de latencia, CPU o RAM sin criterio aprobado y mas evidencia.
- Energia: NO MEDIDA en vatios ni julios. No afirmar presupuesto energetico cumplido.
- Linux con bateria fisica y escenarios especiales: no certificados por los runners CI.
- Esta fase C no modifica Rust, Cargo.lock, dependencias, CI, HardwareInventory V1, politicas ni configuracion del equipo.

Decision C: documentar la evidencia y conservar TASK-002 ABIERTA. La CI del commit C se comprueba despues del push; el resultado no implica aceptacion final automatica.

## 26. Decision final - cierre acotado de TASK-002 (2026-11-02)

Estado: CIERRE ACOTADO ACEPTADO EXPRESAMENTE POR EL PROPIETARIO; verificacion CI del commit documental de cierre obligatoria despues del push. Esta decision posterior actualiza el estado de la TASK sin alterar las propuestas ni los registros historicos anteriores.

Decision literal: `ACEPTO EL CIERRE ACOTADO DE TASK-002 Y AUTORIZO DOCUMENTACIÃ“N, COMMIT Y PUSH.` El propietario acepta expresamente que no estan certificados los presupuestos universales de rendimiento, el consumo energetico, la deteccion fisica de suspension/reanudacion ni Linux con bateria fisica.

### Entrega aceptada

- Telemetria local bajo demanda, solo lectura, con `TelemetrySnapshot` separado de `HardwareInventory` version 1.
- Memoria visible y disponible, uso global de CPU cuando la evidencia permite afirmarlo, fuente de alimentacion y porcentaje de bateria cuando hay datos validos; errores, ausencia y ambiguedad representados explicitamente.
- Captura nativa Windows/Linux en los alcances aceptados A, B1, B2, B3; herramientas B4 para medir latencia del proceso colector, tiempo de CPU del proceso y memoria residente pico.
- Sin servicio residente, historial persistente, telemetria de red, privilegios elevados, politicas, acciones correctivas, ni modificaciones de configuracion del equipo.

### Evidencia y limites de certificacion

- A, B1, B2, B3 y B4 aceptadas previamente. Fase C auditada y publicada en `5e1692b`; GitHub Actions `41501919510` (Windows/Ubuntu x Rust 1.88/stable: 4/4; smoke Linux B4: 2/2), comprobada en la revision de cierre.
- Presupuestos universales de latencia, CPU y memoria: **NO CERTIFICADOS**. Las mediciones B4 del equipo Windows no fijan umbrales contractuales.
- Consumo energetico real en vatios/julios: **NO MEDIDO NI CERTIFICADO**.
- Deteccion fisica de suspension y reanudacion: **NO CERTIFICADA**; las pruebas de discontinuidades no demuestran cobertura universal del evento fisico.
- Linux con bateria fisica: **NO CERTIFICADO** por la CI en runners Ubuntu.
- Tampoco se garantiza disponibilidad de todas las fuentes de telemetria en todos los equipos, ni cobertura global Windows con multiples grupos CPU cuando la fuente no la demuestra.

### Puerta de publicacion y continuidad

La autorizacion conjunta cubre exclusivamente este cierre documental, un commit y su push tras preflight de Git/privacidad, verificacion de las rutas y CI base. Tras publicar, se debe comprobar CI del nuevo commit (4/4 y dos smoke Linux). Si falla o no termina, el registro de aceptacion queda publicado, pero la validacion del cierre sigue PENDIENTE y no se declarara PASS final.

TASK-002 se considera **CERRADA CON ALCANCE ACOTADO solo cuando la CI del commit de cierre sea PASS**. Esta decision no autoriza automaticamente nuevas fases, dependencias, monitorizacion continua ni cambios del sistema.
