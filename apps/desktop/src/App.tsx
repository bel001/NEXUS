import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

type Key = 'cpu' | 'memory' | 'power' | 'inventory' | 'recommendations';
type RecordValue = Record<string, unknown>;
type State = { data?: unknown; error?: string; time?: string };
const sections: { key: Key; title: string; subtitle: string; glyph: string; command: string }[] = [
  { key: 'cpu', title: 'Procesador', subtitle: 'Muestra global bajo demanda', glyph: '◉', command: 'read_cpu' },
  { key: 'memory', title: 'Memoria', subtitle: 'RAM visible y disponible', glyph: '▤', command: 'read_memory' },
  { key: 'power', title: 'Alimentación', subtitle: 'Fuente y batería', glyph: 'ϟ', command: 'read_power' },
  { key: 'inventory', title: 'Inventario', subtitle: 'Resumen sin identificadores', glyph: '▦', command: 'read_inventory_summary' },
  { key: 'recommendations', title: 'Recomendaciones', subtitle: 'Reglas deterministas', glyph: '✧', command: 'read_recommendations' },
];
function obj(v: unknown): RecordValue { return v !== null && typeof v === 'object' && !Array.isArray(v) ? v as RecordValue : {}; }
function number(v: unknown): number | undefined { return typeof v === 'number' && Number.isFinite(v) ? v : undefined; }
function reading(v: unknown): { value?: unknown; status: string } {
  const r = obj(v);
  return { value: r.status === 'available' ? r.value : undefined, status: typeof r.status === 'string' ? r.status : 'unavailable' };
}
function formatNum(v: number | undefined, suffix: string): string { return v === undefined ? '—' : `${v.toLocaleString('es-PE', { maximumFractionDigits: 1 })}${suffix}`; }
function metric(data: unknown, key: Key): { main: string; small: string; status: string } {
  const r = obj(data);
  if (key === 'cpu') {
    const raw = obj(r.cpu_usage); const v = reading(raw.usage_percent); const n = number(v.value);
    return { main: formatNum(n, '%'), small: raw.cpu_scope === 'system_total' ? 'Uso global · muestra única' : 'Alcance global no confirmado', status: v.status };
  }
  if (key === 'memory') {
    const total = reading(r.memory_total_visible_bytes); const available = reading(r.memory_available_bytes);
    const t = number(total.value); const a = number(available.value);
    return { main: formatNum(a === undefined ? undefined : a / (1024 ** 3), ' GiB'), small: t === undefined ? 'Total no disponible' : `De ${formatNum(t / (1024 ** 3), ' GiB')} visibles`, status: a === undefined ? available.status : 'available' };
  }
  if (key === 'power') {
    const p = reading(r.power_source); const b = reading(r.battery_percent);
    return { main: p.value === 'ac' ? 'Conectado' : p.value === 'battery' ? 'Batería' : '—', small: number(b.value) === undefined ? `Batería: ${b.status}` : `Batería: ${formatNum(number(b.value), '%')}`, status: p.status };
  }
  if (key === 'inventory') {
    const cpu = typeof r.cpu_status === 'string' ? r.cpu_status : 'unavailable';
    const storage = number(r.storage_count);
    return { main: cpu === 'available' ? 'Detectado' : '—', small: storage === undefined ? 'Almacenamiento: no disponible' : `${storage} unidades de almacenamiento`, status: cpu };
  }
  const recs = Array.isArray(r.recommendations) ? r.recommendations : [];
  return { main: typeof r.mode === 'string' && r.mode === 'read_only' ? String(recs.length) : '—', small: 'Sugerencias, no acciones', status: r.mode === 'read_only' ? 'available' : 'unavailable' };
}
const statusLabel = (s: string) => ({ available: 'Disponible', unavailable: 'No disponible', unsupported: 'No soportado', permission_denied: 'Sin permiso', error: 'Error' } as Record<string, string>)[s] ?? 'Sin medir';
function Details({ section, value }: { section: Key; value: unknown }) {
  const d = obj(value);
  if (section === 'recommendations') {
    const recs = Array.isArray(d.recommendations) ? d.recommendations : [];
    const missing = Array.isArray(d.unavailable_evidence) ? d.unavailable_evidence.filter((v): v is string => typeof v === 'string') : [];
    return <div className="details"><h3>Resultado de políticas</h3><p className="muted">Una sola captura, sin inferencia de carga sostenida ni acciones automáticas.</p>
      {d.mode !== 'read_only' ? <p>El contrato de solo lectura no fue confirmado.</p> : recs.length === 0 ? <p className="empty">No se emitieron recomendaciones para esta muestra.</p> : recs.map((item, i) => {
        const rec = obj(item);
        return <div className="advice" key={i}><div className="advice-top"><strong>{typeof rec.code === 'string' ? rec.code.replaceAll('_', ' ') : 'Recomendación'}</strong><span>SOLO LECTURA</span></div><p>{typeof rec.explanation === 'string' ? rec.explanation : 'Explicación no disponible'}</p></div>;
      })}
      {missing.length > 0 && <p className="muted">Evidencia ausente: {missing.join(', ')}</p>}
    </div>;
  }
  if (section === 'inventory') return <div className="details"><h3>Estado del inventario</h3><div className="detail-grid">{(['cpu','memory','gpus','storage','batteries'] as const).map(k => <div className="detail-item" key={k}><small>{k.toUpperCase()}</small><strong>{statusLabel(String(d[`${k}_status`] ?? 'unavailable'))}</strong></div>)}</div><p className="muted">NEXUS no expone nombres, números de serie ni identificadores en esta vista.</p></div>;
  const report = section === 'cpu' ? [{ label: 'Uso', raw: obj(d.cpu_usage).usage_percent }] : section === 'memory' ? [
    { label: 'Total visible', raw: d.memory_total_visible_bytes }, { label: 'Disponible', raw: d.memory_available_bytes }
  ] : [{ label: 'Fuente', raw: d.power_source }, { label: 'Batería', raw: d.battery_percent }];
  return <div className="details"><h3>Calidad de evidencia</h3><div className="detail-grid">{report.map(x => <div className="detail-item" key={x.label}><small>{x.label.toUpperCase()}</small><strong>{statusLabel(reading(x.raw).status)}</strong></div>)}</div><p className="muted">Los valores no disponibles no se sustituyen por cero. Sin historial ni medición de energía.</p></div>;
}
export default function App() {
  const [selected, setSelected] = useState<Key>('cpu');
  const [states, setStates] = useState<Partial<Record<Key, State>>>({});
  const [busy, setBusy] = useState<Key | 'all' | null>(null);
  const [autoIntervalSeconds, setAutoIntervalSeconds] = useState<0 | 5 | 10 | 30>(0);
  const inFlight = useRef(false);
  const readSection = useCallback(async (key: Key): Promise<void> => {
    try {
      const command = sections.find(s => s.key === key)!.command;
      const data: unknown = await invoke(command);
      if (obj(data).schema_version !== 1) throw new Error('invalid_schema_version');
      setStates(old => ({ ...old, [key]: { data, time: new Date().toLocaleTimeString('es-PE') } }));
    } catch (reason) {
      const raw = typeof reason === 'string' ? reason : reason instanceof Error ? reason.message : '';
      const known = ['serialization_failed', 'invalid_json', 'unsupported_inventory_schema', 'invalid_schema_version'];
      const diagnostic = known.find(code => raw.includes(code)) ??
        (/command.*not found/i.test(raw) ? 'command_not_found' : 'unclassified_invoke_error');
      setStates(old => ({ ...old, [key]: { error: `Lectura ${key} fallida. Código: ${diagnostic}.` } }));
    }
  }, []);

  const refresh = useCallback(async (key: Key): Promise<void> => {
    if (inFlight.current) return;
    inFlight.current = true;
    setBusy(key);
    try { await readSection(key); }
    finally { inFlight.current = false; setBusy(null); }
  }, [readSection]);

  const refreshAll = useCallback(async (): Promise<void> => {
    if (inFlight.current) return;
    inFlight.current = true;
    setBusy('all');
    try {
      for (const section of sections) {
        await readSection(section.key);
      }
    } finally { inFlight.current = false; setBusy(null); }
  }, [readSection]);

  useEffect(() => {
    if (autoIntervalSeconds === 0) return;
    let timer: number | undefined;
    let stopped = false;
    const isActive = () => document.visibilityState === 'visible' && document.hasFocus();
    const cancel = () => {
      if (timer !== undefined) { window.clearTimeout(timer); timer = undefined; }
    };
    const schedule = () => {
      if (stopped || !isActive() || timer !== undefined) return;
      timer = window.setTimeout(async () => {
        timer = undefined;
        if (stopped || !isActive()) return;
        await refreshAll();
        if (!stopped) schedule();
      }, autoIntervalSeconds * 1000);
    };
    const onActivity = () => { cancel(); schedule(); };
    document.addEventListener('visibilitychange', onActivity);
    window.addEventListener('focus', onActivity);
    window.addEventListener('blur', onActivity);
    schedule();
    return () => {
      stopped = true;
      cancel();
      document.removeEventListener('visibilitychange', onActivity);
      window.removeEventListener('focus', onActivity);
      window.removeEventListener('blur', onActivity);
    };
  }, [autoIntervalSeconds, refreshAll]);
  const current = sections.find(s => s.key === selected)!;
  const selectedState = states[selected];
  const activeMetric = selectedState?.data ? metric(selectedState.data, selected) : undefined;
  return <div className="shell">
    <aside className="sidebar">
      <div className="brand"><div className="brand-mark">N</div><div><strong>NEXUS</strong><small>ADAPTIVE ENGINE</small></div></div>
      <div className="nav-label">ESPACIO DE TRABAJO</div>
      <nav aria-label="Secciones">{sections.map(s => <button key={s.key} className={'nav-item ' + (selected === s.key ? 'active' : '')} onClick={() => setSelected(s.key)} aria-current={selected === s.key ? 'page' : undefined}><span className="nav-icon">{s.glyph}</span><span>{s.title}</span><span className="nav-chevron">›</span></button>)}</nav>
      <div className="sidebar-bottom"><span className="safe-dot"/> MODO OBSERVACIÓN<div className="bottom-note">Sin cambios al sistema</div></div>
    </aside>
    <main className="main">
      <header className="topbar"><div className="breadcrumb">PLATAFORMA / <span>RESUMEN</span></div><div className="top-right"><span className="local-indicator"/> ENTORNO LOCAL <span className="version">PRE-ALPHA · v0.1</span></div></header>
      <div className="content">
        <div className="overline">DIAGNÓSTICO · SOLO LECTURA</div><h1>Tu equipo, con más claridad<span>.</span></h1><p className="intro">Observa indicadores reales y recibe recomendaciones conservadoras. Actualiza manualmente o activa lecturas periódicas mientras NEXUS esté visible y en primer plano.</p>
        <section className="overview"><div className="section-header"><h2>Estado del sistema</h2><div className="overview-actions"><label className="auto-interval-control" htmlFor="nexus-auto-refresh">Automática
  <select id="nexus-auto-refresh" aria-label="Intervalo de actualización automática" value={autoIntervalSeconds} onChange={event => setAutoIntervalSeconds(Number(event.target.value) as 0 | 5 | 10 | 30)}>
    <option value={0}>Desactivada</option><option value={5}>Cada 5 s</option><option value={10}>Cada 10 s</option><option value={30}>Cada 30 s</option>
  </select></label><button className="refresh" type="button" onClick={()=>void refreshAll()} disabled={busy !== null}><span>↻</span> {busy === 'all' ? 'Midiendo todo…' : 'Actualizar todo'}</button></div></div><p className="auto-refresh-note">Auto: solo con esta ventana visible y activa; sin historial ni procesos residentes.</p><div className="metric-grid">{sections.slice(0,4).map(s => {
          const st = states[s.key]; const met = st?.data ? metric(st.data,s.key) : undefined;
          return <button key={s.key} className={'metric-card ' + (selected===s.key?'selected':'')} onClick={()=>setSelected(s.key)}><div className="metric-head"><span className="metric-icon">{s.glyph}</span><span className="metric-status">{met ? statusLabel(met.status) : 'SIN MEDIR'}</span></div><div className="metric-name">{s.title}</div><div className="metric-value">{met?.main ?? '—'}</div><div className="metric-foot">{met?.small ?? s.subtitle}</div></button>;
        })}</div></section>
        <section className="workspace"><div className="workspace-heading"><div><div className="section-kicker">CENTRO DE OBSERVACIÓN</div><h2>{current.title}</h2><p>{current.subtitle}</p></div><button className="refresh" onClick={()=>void refresh(selected)} disabled={busy !== null}><span>{busy===selected?'◌':'↻'}</span> {busy===selected?'Midiendo…':'Actualizar ahora'}</button></div>
          {!selectedState?.data && !selectedState?.error ? <div className="waiting"><span className="waiting-icon">◎</span><h3>Listo para una medición</h3><p>Presiona «Actualizar ahora» para consultar esta sección. También puedes activar la actualización automática desde el resumen.</p></div> : <div className="report-body">
            {selectedState?.error && <div role="alert" className="error-box">{selectedState.error}</div>}
            {activeMetric && <div className="report-summary"><div><span>ÚLTIMA LECTURA</span><strong>{activeMetric.main}</strong><small>{activeMetric.small}</small></div><div className="readtime">{selectedState?.time ? `Consultado a las ${selectedState.time}` : 'Sin lectura'}</div></div>}
            {selectedState?.data != null && <Details section={selected} value={selectedState.data}/>}
          </div>}
        </section>
        <footer className="footer"><span>© 2026 NEXUS · Motor de observación local</span><span>Sin telemetría remota · Sin optimización automática</span></footer>
      </div>
    </main>
  </div>;
}
