# Modelo de amenazas y seguridad

## Activos a proteger

- Estabilidad del sistema operativo.
- Datos y procesos del usuario.
- Configuraciones de energía.
- Privilegios del usuario.
- Integridad de actualizaciones.
- Historial local de acciones.

## Amenazas

- UI comprometida que intenta ejecutar operaciones arbitrarias.
- Dependencia maliciosa o comprometida.
- Acción incompatible con un modelo de hardware.
- Dos gestores energéticos sobrescribiéndose.
- Error del agente al restaurar una configuración.
- Datos de telemetría sensibles.
- Actualización manipulada.

## Controles

1. Agente sin privilegios elevados por defecto.
2. IPC local autenticado y con protocolo versionado.
3. Allowlist de acciones tipadas; nunca shell genérico.
4. Capabilities de Tauri mínimas cuando exista UI.
5. Consentimiento explícito para acciones importantes.
6. Registro de estado previo y rollback.
7. Firma y verificación de actualizaciones.
8. Datos locales y telemetría remota desactivada por defecto.
9. No recopilar contenido de documentos, teclas ni historial de navegación.
10. Pruebas de fallos, desconexión, suspensión y reanudación.

## Niveles de acción

- L0: lectura, automática.
- L1: recomendación, automática.
- L2: cambio reversible de bajo riesgo, requiere consentimiento inicial/configuración.
- L3: cambio avanzado dependiente de fabricante, confirmación explícita y validación específica.
- Prohibido en el producto: desactivar seguridad, manipular firmware/voltajes, eliminar archivos del sistema o ejecutar scripts arbitrarios.

## Tauri

La UI solo tendrá los comandos y permisos estrictamente necesarios. Las capacidades de Tauri reducen la superficie expuesta al frontend, pero no sustituyen la revisión de seguridad del código Rust.

Referencia: https://v2.tauri.app/security/capabilities/

## Controles de Hardware Discovery

TASK-001 pertenece a L0 y es estrictamente de lectura:

- Windows usa `SELECT` predefinidos y no acepta texto externo;
- no se invocan métodos WMI ni PowerShell;
- Linux solo lee interfaces locales y nunca escribe en sysfs;
- no se ejecutan comandos o scripts auxiliares;
- no se solicita elevación para la captura normal;
- no hay red ni telemetría;
- no se serializan devpaths, rutas internas, identificadores de correlación,
  números de serie ni otros identificadores innecesarios;
- los errores se estructuran y sanitizan antes de incluirse en el inventario.

La especificación aprobada está en
`docs/architecture/hardware-discovery.md`. Estos controles todavía deben
implementarse y validarse.
