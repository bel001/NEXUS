# TASK-002 B4 — Metodologia de coste del colector (B4 aceptada)

Estado: FASE B4 ACEPTADA FORMALMENTE (2026-11-02). TASK-002 con cierre acotado aceptado; CI del commit de cierre sujeta a verificacion pospush; sin presupuestos de rendimiento ni energia certificados.

## Objetivo

Medir el coste del proceso `adaptive-agent` al solicitar **una** medicion de memoria, CPU o alimentacion. No mide consumo energetico en vatios o julios, ni altera el contrato `TelemetrySnapshot` o `HardwareInventory`.

## Procedimiento reproducible

- Compilar `adaptive-agent` con Rust 1.88, perfil `release`, sin nuevas dependencias.
- Ejecutar cada modo `--telemetry-memory`, `--telemetry-cpu` y `--telemetry-power` por separado, en proceso nuevo.
- Descartar dos ejecuciones de calentamiento; tomar diez mediciones por modo.
- Usar reloj monotono desde el lanzamiento hasta la terminacion, incluyendo arranque del proceso y salida JSON.
- Medir tiempo de CPU del proceso, **no** porcentaje de uso global de CPU.
- Medir memoria residente maxima (working set maximo en Windows, RSS maximo reportado por GNU time en Linux).
- Reportar p50 y p95 mediante nearest rank, y cantidades de ejecuciones; no confundir tiempos o memoria con energia.
- No imprimir, publicar ni conservar la salida JSON de telemetria; validar la estructura en memoria en Windows.
- Medir presupuestos de rendimiento fuera de CI. La CI Linux solo ejecuta una comprobacion funcional del medidor; los runners compartidos no fijan umbrales.

## Herramientas

- Windows: `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/benchmarks/telemetry-b4-windows.ps1 -Executable target/release/adaptive-agent.exe`. Usa `System.Diagnostics.Process` y `Stopwatch` del sistema. El reporte numerico se escribe **solo** en `%TEMP%` con nombre unico y no contiene identificadores.
- Linux: `bash scripts/benchmarks/telemetry-b4-linux.sh target/release/adaptive-agent`. Requiere GNU `/usr/bin/time`, `bash`, `awk`, `sed` y `sort` instalados. No escribe reportes persistentes; su espacio temporal se elimina al finalizar.

## Limitaciones

- Los resultados incluyen arranque de proceso, carga del runtime, invocaciones WMI/sysfs y serializacion JSON; no son latencia aislada de cada API.
- La memoria residente incluye paginas compartidas. No equivale a memoria privada ni a energia.
- La contabilidad de CPU depende de la granularidad de cada sistema operativo; algunas muestras muy rapidas pueden registrar cero.
- La latencia WMI y el coste varian entre maquinas, estado termico, carga y fuente de alimentacion.
- Ni este benchmark ni las pruebas CI certifican funcionamiento tras suspension o presupuestos energeticos.
- No establecer umbrales de aceptacion sin mediciones repetidas y aprobacion expresa.
- B4 no autoriza un servicio residente, acciones del sistema, redes, dependencias, cambios al inventario ni nuevas fases.

## Evidencia

Windows: medicion nativa local completada, con 2 calentamientos y 10 muestras por modo. Los resultados numericos y el reporte detallado permanecen en TEMP, sin versionar.

Linux: se habilita en Ubuntu CI una prueba funcional con 1 calentamiento y 5 muestras por modo. Los resultados del runner no representan un presupuesto de latencia o consumo; su exito debe confirmarse en el commit publicado.

Aceptacion B4: COMPLETADA (2026-11-02), commit 8ff0e52 y CI 41498563272 (4/4 y smoke Linux 2/2). Presupuesto energetico: NO MEDIDO NI CERTIFICADO. No se han medido vatios ni julios. No publicar informes privados ni identificar equipos. Si se desea publicar una tabla resumida, revisar primero su privacidad. No incluir rutas personales, nombres de usuarios, seriales ni valores de telemetria del equipo.
