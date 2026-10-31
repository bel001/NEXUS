# Arquitectura general

## 1. Objetivo

Construir una plataforma local de diagnóstico y optimización adaptativa que mejore el aprovechamiento del hardware y la eficiencia energética. El sistema debe ser transparente, medible, seguro y reversible.

No se promete equivalencia física con Apple Silicon ni superioridad universal frente a Mac. Se optimiza cada equipo dentro de sus límites térmicos, eléctricos y de hardware.

## 2. Principios

- Read-only primero: inventariar y medir antes de modificar.
- Capabilities-first: no asumir que una función existe.
- Event-driven: reaccionar a eventos del sistema cuando sea posible; sondear solo lo imprescindible.
- Least privilege: el agente no debe ejecutarse como administrador/root por defecto.
- No competir con el sistema operativo ni con otros gestores de energía.
- Separar observación, decisión y acción.
- Toda acción debe ser auditable, verificable y reversible.
- Offline-first: no se requiere cuenta ni conexión remota.

## 3. Procesos

### Desktop UI (futuro)
Tauri 2 + React + TypeScript. Solo presenta datos, solicita operaciones declarativas y muestra consentimiento. No ejecuta comandos del sistema.

### Agent
Proceso de sesión del usuario, ligero. Recopila telemetría de bajo coste y expone un IPC local autenticado. No mantiene la UI abierta.

### Core
Lógica de dominio, normalización, reglas, planificación, validación y almacenamiento. El Core no conoce detalles Win32, sysfs ni D-Bus.

### Platform adapters
Implementaciones específicas de Windows y Linux. Devuelven capacidades y datos con procedencia y estado de disponibilidad.

## 4. Flujo de datos

1. El adaptador detecta hardware y capacidades.
2. Telemetry produce snapshots con timestamp y procedencia.
3. El clasificador identifica contexto (AC/batería, foreground/idle, carga).
4. Policy Engine genera una recomendación/plan.
5. En modo read-only el plan se muestra, nunca se aplica.
6. En fases posteriores, Action Engine valida permisos y compatibilidad.
7. Transaction/Rollback registra estado previo, aplica, verifica y revierte si falla.
8. Benchmark compara condiciones equivalentes antes/después.

## 5. Límites del MVP

Incluye:
- Workspace Rust.
- Contratos iniciales.
- Agente de consola scaffold.
- Descubrimiento de hardware read-only como primera tarea.
- Windows y Linux.

No incluye:
- Cambios de energía.
- EcoQoS aplicado.
- Control de ventiladores.
- Voltajes/frecuencias.
- Limpieza automática de RAM.
- IA/ML.
- Tauri UI.
- Telemetría remota.
