// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub(crate) const TEMPLATE: &str = r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Aptos framework deprecation evidence</title>
<style>
:root { color-scheme: light; --ink:#16202a; --muted:#617080; --line:#dce3e8; --paper:#fff; --canvas:#f3f6f8; --accent:#2457d6; --danger:#a83232; --warn:#986600; --ok:#217047; }
* { box-sizing:border-box; }
body { margin:0; background:var(--canvas); color:var(--ink); font:14px/1.45 ui-sans-serif,system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif; }
main { max-width:1500px; margin:auto; padding:28px; }
h1 { margin:0; font-size:27px; letter-spacing:-.02em; }
h2 { font-size:18px; margin:0 0 12px; }
.subtitle { color:var(--muted); margin:5px 0 22px; }
.notice { background:#fff9dd; border:1px solid #eadc93; border-radius:8px; padding:12px 14px; margin-bottom:18px; }
.cards { display:grid; grid-template-columns:repeat(auto-fit,minmax(170px,1fr)); gap:10px; margin-bottom:18px; }
.card,.panel { background:var(--paper); border:1px solid var(--line); border-radius:9px; box-shadow:0 1px 2px #14202b0a; }
.card { padding:13px; }
.card .label { display:block; color:var(--muted); font-size:12px; }
.card .value { display:block; font-size:21px; font-weight:700; margin-top:2px; }
.card .value.time { font-size:13px; line-height:1.5; overflow-wrap:anywhere; }
.panel { padding:16px; margin-bottom:16px; }
.controls { display:grid; grid-template-columns:minmax(240px,2fr) repeat(3,minmax(140px,1fr)); gap:10px; align-items:end; }
label { color:var(--muted); font-size:12px; }
input,select { width:100%; margin-top:4px; padding:8px 9px; border:1px solid #bdc7d0; border-radius:6px; background:white; color:var(--ink); }
.legend { display:flex; flex-wrap:wrap; gap:14px; margin-top:12px; color:var(--muted); font-size:12px; }
.dot { width:9px; height:9px; border-radius:50%; display:inline-block; margin-right:5px; }
.table-wrap { overflow:auto; max-height:70vh; border:1px solid var(--line); border-radius:7px; }
table { width:100%; border-collapse:collapse; background:white; }
th { position:sticky; top:0; z-index:1; background:#edf2f5; text-align:left; font-size:12px; color:#4d5a66; padding:9px; border-bottom:1px solid #cbd4dc; white-space:nowrap; }
td { padding:9px; border-bottom:1px solid #e7ecef; vertical-align:top; }
tbody tr:hover { background:#f7faff; }
code { font:12px/1.4 ui-monospace,SFMono-Regular,Menlo,Consolas,monospace; }
.number { text-align:right; font-variant-numeric:tabular-nums; }
.function-column { width:320px; max-width:320px; }
.function-column .link { max-width:100%; text-align:left; }
.function-column code { overflow-wrap:anywhere; }
.module-row:hover { background:transparent; }
.module-row td { background:#e7edf2; border-bottom:1px solid #c5d0d9; padding:8px 10px; }
.module-toggle { display:flex; align-items:center; gap:8px; width:100%; border:0; padding:0; background:none; color:var(--ink); cursor:pointer; text-align:left; font:inherit; }
.module-toggle code { font-weight:700; }
.module-chevron { width:12px; color:var(--muted); }
.module-summary { margin-left:auto; color:var(--muted); font-size:12px; font-weight:400; }
.table-heading { display:flex; align-items:center; justify-content:space-between; gap:12px; margin-bottom:12px; }
.table-heading h2 { margin:0; }
.table-heading .meta { margin:3px 0 0; }
.module-control { border:1px solid #bdc7d0; border-radius:6px; padding:6px 9px; background:white; color:var(--ink); cursor:pointer; font:inherit; font-size:12px; }
.module-control:disabled { cursor:default; opacity:.55; }
.badge { display:inline-block; border-radius:999px; padding:2px 7px; font-size:11px; font-weight:650; white-space:nowrap; }
.unused-external { color:var(--danger); background:#fde7e7; }
.rare-external { color:var(--warn); background:#fff0c9; }
.unused-internal { color:#674c89; background:#f1e9fb; }
.active { color:var(--ok); background:#e4f4eb; }
.meta { color:var(--muted); font-size:12px; }
button.link { border:0; background:none; color:var(--accent); padding:0; cursor:pointer; font:inherit; text-decoration:underline; }
#details[hidden] { display:none; }
#details { position:fixed; z-index:20; width:min(920px,calc(100vw - 32px)); max-height:min(70vh,680px); overflow:auto; margin:0; padding:16px; border-color:#aebbc6; box-shadow:0 12px 35px #14202b33; }
.detail-header { display:flex; align-items:flex-start; justify-content:space-between; gap:12px; margin-bottom:12px; }
.detail-header h2 { margin:0; overflow-wrap:anywhere; }
.detail-close { flex:0 0 auto; width:28px; height:28px; border:1px solid #bdc7d0; border-radius:6px; background:white; color:var(--muted); cursor:pointer; font:18px/1 ui-sans-serif,system-ui,sans-serif; }
.detail-grid { display:grid; grid-template-columns:repeat(4,minmax(120px,1fr)); gap:8px 18px; margin-bottom:12px; }
.detail-grid .value { display:block; font-weight:650; overflow-wrap:anywhere; }
.paths { max-height:330px; overflow:auto; }
.empty { padding:25px; text-align:center; color:var(--muted); }
@media (max-width:900px) { main{padding:15px}.cards{grid-template-columns:repeat(2,1fr)}.controls{grid-template-columns:1fr 1fr}.detail-grid{grid-template-columns:1fr 1fr}#details{width:calc(100vw - 20px);max-height:calc(100vh - 20px)} }
</style>
</head>
<body>
<main>
  <h1>Framework deprecation evidence</h1>
  <p class="subtitle" id="subtitle"></p>
  <div class="notice"><strong>Interpretation:</strong> no observed calls means “not seen in this replay range,” not “safe to remove.” Review longer and representative ranges, source-level dependencies, compatibility policy, and native/runtime coupling before deprecating code.</div>
  <div class="notice" id="usage-detail-warning" hidden></div>
  <section class="cards" id="cards"></section>
  <section class="panel">
    <div class="controls">
      <label>Search function, module, or address<input id="search" type="search" placeholder="coin, 0x1::account, transfer..."></label>
      <label>Evidence class<select id="class-filter"><option value="candidates">External deprecation candidates</option><option value="unused">All unobserved functions</option><option value="all">All functions</option><option value="active">Observed functions</option></select></label>
      <label>Visibility<select id="visibility"><option value="all">All visibility</option><option value="public">Public</option><option value="friend">Friend</option><option value="private">Private</option></select></label>
      <label>Rare threshold (transactions)<input id="threshold" type="number" min="1" value="10"></label>
    </div>
    <div class="legend"><span><i class="dot" style="background:#c64a4a"></i>unobserved external</span><span><i class="dot" style="background:#d29a20"></i>rare external</span><span><i class="dot" style="background:#8c6bb1"></i>unobserved internal</span><span><i class="dot" style="background:#3a9665"></i>active</span></div>
  </section>
  <section class="panel" id="details" role="dialog" aria-labelledby="detail-title" hidden>
    <div class="detail-header"><h2 id="detail-title"></h2><button class="detail-close" id="detail-close" type="button" aria-label="Close function details">×</button></div>
    <div class="detail-grid" id="detail-summary"></div>
    <h2>Calling entry functions</h2>
    <div class="paths table-wrap" id="detail-roots"></div>
    <h2>Immediate call paths</h2>
    <div class="paths table-wrap" id="detail-paths"></div>
  </section>
  <section class="panel">
    <div class="table-heading"><div><h2>Active entry-function callers</h2><p class="meta">Observed entry functions that invoked framework code, grouped by their module address.</p></div><button class="module-control" id="toggle-entrypoint-callers" type="button">Expand all entrypoints</button></div>
    <div class="table-wrap"><table><thead><tr><th>Module address</th><th class="function-column">Entry function</th><th>Outcome</th><th class="number">Transactions</th><th class="number">Framework invocations</th><th>Observed versions</th></tr></thead><tbody id="active-entry-function-callers"></tbody></table></div>
  </section>
  <section class="panel">
    <div class="table-heading"><h2 id="table-title">Functions</h2><button class="module-control" id="toggle-modules" type="button">Collapse all modules</button></div>
    <div class="table-wrap"><table><thead><tr><th>Evidence</th><th class="function-column">Function</th><th>Visibility</th><th class="number">Transactions</th><th class="number">Invocations</th><th class="number">Successful</th><th>Observed versions</th><th>Callers</th></tr></thead><tbody id="function-rows"></tbody></table></div>
  </section>
</main>
<script id="report-data" type="application/json">__FRAMEWORK_USAGE_REPORT__</script>
<script>
"use strict";
const report = JSON.parse(document.getElementById("report-data").textContent);
const nf = new Intl.NumberFormat();
const utc = usecs => new Date(Number(usecs) / 1000).toISOString();
const esc = value => String(value ?? "").replace(/[&<>"']/g, c => ({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"}[c]));
const shortAddress = address => {
  const value = String(address ?? "");
  const hex = value.replace(/^0x/i, "");
  if (!/^[0-9a-f]+$/i.test(hex)) return value;
  const compact = hex.replace(/^0+/, "") || "0";
  return compact.length > 16 ? `${compact.slice(0, 8)}..${compact.slice(-4)}` : `0x${compact}`;
};
const rawModuleName = m => m ? `${m.address}::${m.name}` : "<script>";
const rawFunctionName = f => f ? `${rawModuleName(f.module_id)}::${f.function_name}` : "<entrypoint>";
const moduleName = m => m ? `${shortAddress(m.address)}::${m.name}` : "<script>";
const functionName = f => f ? `${moduleName(f.module_id)}::${f.function_name}` : "<entrypoint>";
const entrypointFunctionName = f => f?.module_id ? `${f.module_id.name}::${f.function_name}` : "<entrypoint>";
const keyOf = f => rawFunctionName(f);
const external = f => f.visibility !== "private" || f.is_entry;

const functions = new Map();
for (const f of report.functions) {
  const id = `${rawModuleName(f.module_id)}::${f.function_name}`;
  const displayId = `${moduleName(f.module_id)}::${f.function_name}`;
  functions.set(id, {...f, id, displayId, invocations:0, transactions:0, successful:0, first:null, last:null, roots:[], paths:[], callers:new Set()});
}
for (const u of report.function_usage) {
  const f = functions.get(keyOf(u.callee));
  if (!f) continue;
  f.invocations += u.invocation_count;
  f.transactions += u.transaction_count;
  if (u.outcome === "success") f.successful += u.invocation_count;
  f.first = f.first === null ? u.first_version : Math.min(f.first, u.first_version);
  f.last = f.last === null ? u.last_version : Math.max(f.last, u.last_version);
}
for (const r of report.root_function_usage ?? []) {
  const f = functions.get(keyOf(r.callee));
  if (f) f.roots.push(r);
}
for (const p of report.usage) {
  const f = functions.get(keyOf(p.callee));
  if (!f) continue;
  f.paths.push(p);
  if (p.caller?.module_id?.address) f.callers.add(p.caller.module_id.address);
}
const all = [...functions.values()];
const modules = new Map();
for (const f of all) {
  const id = rawModuleName(f.module_id);
  if (!modules.has(id)) modules.set(id, {id, displayId:moduleName(f.module_id), functions:[]});
  modules.get(id).functions.push(f);
}
// Every expandable group starts collapsed: the report's first view should surface the
// least-observed modules and their aggregate evidence without overwhelming the reader.
const collapsedModules = new Set(modules.keys());
const expandedEntrypointAddresses = new Set();
let visibleModuleIds = [];
let visibleEntrypointAddresses = [];

function evidence(f, threshold) {
  if (f.invocations === 0 && external(f)) return {rank:0, cls:"unused-external", text:"Unobserved external"};
  if (f.transactions <= threshold && f.invocations > 0 && external(f)) return {rank:1, cls:"rare-external", text:"Rare external"};
  if (f.invocations === 0) return {rank:2, cls:"unused-internal", text:"Unobserved internal"};
  return {rank:3, cls:"active", text:"Active"};
}

function renderCards() {
  const unusedExternal = all.filter(f => f.invocations === 0 && external(f)).length;
  const unused = all.filter(f => f.invocations === 0).length;
  const values = [
    ["Ledger range", `${nf.format(report.start_version)}–${nf.format(report.end_version)}`, ""],
    ["UTC range", `${utc(report.start_timestamp_usecs)} → ${utc(report.end_timestamp_usecs)}`, "time"],
    ["Transactions replayed", nf.format(report.processed_transaction_count), ""],
    ["User payload records", nf.format(report.transaction_usage_records), ""],
    ["Framework functions", nf.format(all.length), ""],
    ["Unobserved external", nf.format(unusedExternal), ""],
    ["All unobserved", nf.format(unused), ""]
  ];
  document.getElementById("cards").innerHTML = values.map(([label,value,cls]) => `<div class="card"><span class="label">${esc(label)}</span><span class="value ${cls}">${esc(value)}</span></div>`).join("");
  const recordedAt = report.report_generated_at_utc ? new Date(report.report_generated_at_utc).toISOString() : "recording date unavailable";
  document.getElementById("subtitle").textContent = `Recorded ${recordedAt} · Replay ${nf.format(report.start_version)}–${nf.format(report.end_version)} · schema ${report.schema_version} · build ${report.git_sha || "unknown"}`;
  const truncationNotices = [];
  if (report.usage_detail_truncated) {
    const rowLimit = report.merged_usage_detail_row_limit ?? report.usage_detail_row_limit;
    truncationNotices.push(`Call-path detail is partial: it is capped at ${nf.format(rowLimit)} rows; ${nf.format(report.dropped_usage_invocation_count)} invocation${report.dropped_usage_invocation_count === 1 ? " was" : "s were"} omitted across ${nf.format(report.dropped_usage_transaction_count)} transaction${report.dropped_usage_transaction_count === 1 ? "" : "s"}. Per-function totals remain complete.`);
  }
  if (report.root_function_usage_truncated) {
    truncationNotices.push(`Calling-entry attribution is partial for functions with more than ${nf.format(report.root_function_row_limit_per_function)} distinct entry/outcome combinations; ${nf.format(report.dropped_root_function_usage_invocation_count)} invocation${report.dropped_root_function_usage_invocation_count === 1 ? " was" : "s were"} omitted across ${nf.format(report.dropped_root_function_usage_transaction_count)} transaction${report.dropped_root_function_usage_transaction_count === 1 ? "" : "s"}.`);
  }
  if (report.active_entry_function_callers_truncated) {
    const rowLimit = report.merged_active_entry_function_caller_row_limit ?? report.active_entry_function_caller_row_limit;
    truncationNotices.push(`Active entry-function callers are partial: they are capped at ${nf.format(rowLimit)} rows; ${nf.format(report.dropped_active_entry_function_framework_invocation_count)} framework invocation${report.dropped_active_entry_function_framework_invocation_count === 1 ? " was" : "s were"} omitted across ${nf.format(report.dropped_active_entry_function_transaction_count)} transaction${report.dropped_active_entry_function_transaction_count === 1 ? "" : "s"}.`);
  }
  if (truncationNotices.length) {
    const warning = document.getElementById("usage-detail-warning");
    warning.hidden = false;
    warning.textContent = truncationNotices.join(" ");
  }
}

function renderActiveEntryFunctionCallers() {
  const groups = new Map();
  for (const caller of report.active_entry_function_callers ?? []) {
    if (!groups.has(caller.address)) groups.set(caller.address, []);
    groups.get(caller.address).push(caller);
  }
  const sections = [...groups].map(([address, callers]) => ({
    address,
    callers,
    invocationCount: callers.reduce((total, caller) => total + caller.framework_invocation_count, 0)
  })).sort((left, right) => right.invocationCount - left.invocationCount || left.address.localeCompare(right.address));
  visibleEntrypointAddresses = sections.map(section => section.address);
  const toggle = document.getElementById("toggle-entrypoint-callers");
  const allExpanded = visibleEntrypointAddresses.length > 0 && visibleEntrypointAddresses.every(address => expandedEntrypointAddresses.has(address));
  toggle.textContent = allExpanded ? "Collapse all entrypoints" : "Expand all entrypoints";
  toggle.disabled = visibleEntrypointAddresses.length === 0;
  const rows = sections.map(({address, callers, invocationCount}) => {
    callers.sort((left, right) => right.transaction_count - left.transaction_count || right.framework_invocation_count - left.framework_invocation_count || entrypointFunctionName(left.entry_function).localeCompare(entrypointFunctionName(right.entry_function)));
    const expanded = expandedEntrypointAddresses.has(address);
    const header = `<tr class="module-row"><td colspan="6"><button class="module-toggle" data-entrypoint-address="${esc(address)}" aria-expanded="${expanded}"><span class="module-chevron">${expanded?"▾":"▸"}</span><code>${esc(address)}</code><span class="module-summary">${nf.format(invocationCount)} framework call${invocationCount === 1 ? "" : "s"}</span></button></td></tr>`;
    const callerRows = callers.map(caller => `<tr><td></td><td class="function-column"><code title="${esc(functionName(caller.entry_function))}">${esc(entrypointFunctionName(caller.entry_function))}</code></td><td>${esc(caller.outcome)}</td><td class="number">${nf.format(caller.transaction_count)}</td><td class="number">${nf.format(caller.framework_invocation_count)}</td><td>${nf.format(caller.first_version)}–${nf.format(caller.last_version)}</td></tr>`).join("");
    return header + (expanded ? callerRows : "");
  });
  document.getElementById("active-entry-function-callers").innerHTML = rows.length ? rows.join("") : `<tr><td colspan="6" class="empty">No module entry functions invoked framework code in this replay range.</td></tr>`;
  for (const button of document.querySelectorAll("button[data-entrypoint-address]")) button.addEventListener("click", () => {
    const address = button.dataset.entrypointAddress;
    if (expandedEntrypointAddresses.has(address)) expandedEntrypointAddresses.delete(address); else expandedEntrypointAddresses.add(address);
    renderActiveEntryFunctionCallers();
  });
}

function renderFunctions() {
  closeDetails();
  const threshold = Math.max(1, Number(document.getElementById("threshold").value) || 10);
  const query = document.getElementById("search").value.trim().toLowerCase();
  const classFilter = document.getElementById("class-filter").value;
  const visibility = document.getElementById("visibility").value;
  const rows = all.filter(f => {
    const e = evidence(f, threshold);
    if (query && !f.id.toLowerCase().includes(query) && !f.displayId.toLowerCase().includes(query)) return false;
    if (visibility !== "all" && f.visibility !== visibility) return false;
    if (classFilter === "candidates" && e.rank > 1) return false;
    if (classFilter === "unused" && f.invocations !== 0) return false;
    if (classFilter === "active" && f.invocations === 0) return false;
    return true;
  }).sort((a,b) => {
    const ea=evidence(a,threshold), eb=evidence(b,threshold);
    return ea.rank-eb.rank || a.transactions-b.transactions || a.invocations-b.invocations || a.displayId.localeCompare(b.displayId);
  });
  const grouped = new Map();
  for (const f of rows) {
    const moduleId = rawModuleName(f.module_id);
    if (!grouped.has(moduleId)) grouped.set(moduleId, []);
    grouped.get(moduleId).push(f);
  }
  const moduleEvidence = module => {
    const unobservedFunctions=module.functions.filter(f => f.invocations === 0).length;
    const invocationCount=module.functions.reduce((total, f) => total + f.invocations, 0);
    const rarelyObserved=invocationCount > 0 && module.functions.every(f => f.invocations === 0 || f.transactions <= threshold);
    return {
      rank:invocationCount === 0 ? 0 : rarelyObserved ? 1 : 2,
      unobservedFunctions,
      invocationCount,
      rarelyObserved
    };
  };
  const groups = [...grouped].sort(([left], [right]) => {
    const leftModule = modules.get(left), rightModule = modules.get(right);
    const leftEvidence = moduleEvidence(leftModule), rightEvidence = moduleEvidence(rightModule);
    // Keep unobserved, rare-only, and regularly observed modules in distinct tiers.
    // Within a tier, surface the broadest deprecation candidates and least-used code first.
    return leftEvidence.rank - rightEvidence.rank
      || rightEvidence.unobservedFunctions - leftEvidence.unobservedFunctions
      || leftEvidence.invocationCount - rightEvidence.invocationCount
      || leftModule.displayId.localeCompare(rightModule.displayId);
  });
  visibleModuleIds = groups.map(([moduleId]) => moduleId);
  const toggle = document.getElementById("toggle-modules");
  const allCollapsed = visibleModuleIds.length > 0 && visibleModuleIds.every(moduleId => collapsedModules.has(moduleId));
  toggle.textContent = allCollapsed ? "Expand all modules" : "Collapse all modules";
  toggle.disabled = visibleModuleIds.length === 0;
  document.getElementById("table-title").textContent = `Functions (${nf.format(rows.length)} shown)`;
  const functionRow = f => {
    const e=evidence(f,threshold);
    const flags=[f.is_entry?"entry":"",f.is_native?"native":"",f.type_parameter_count?`${f.type_parameter_count} type param${f.type_parameter_count===1?"":"s"}`:""] .filter(Boolean).join(" · ");
    return `<tr><td><span class="badge ${e.cls}">${esc(e.text)}</span></td><td class="function-column"><button class="link" data-function="${esc(f.id)}"><code title="${esc(f.id)}">${esc(f.function_name)}</code></button>${flags?`<div class="meta">${esc(flags)}</div>`:""}</td><td>${esc(f.visibility)}</td><td class="number">${nf.format(f.transactions)}</td><td class="number">${nf.format(f.invocations)}</td><td class="number">${nf.format(f.successful)}</td><td>${f.first===null?"Never":`${nf.format(f.first)}–${nf.format(f.last)}`}</td><td>${nf.format(f.callers.size)} address${f.callers.size===1?"":"es"}</td></tr>`;
  };
  document.getElementById("function-rows").innerHTML = groups.length ? groups.map(([moduleId, visibleFunctions]) => {
    const module = modules.get(moduleId);
    const observed = module.functions.filter(f => f.invocations > 0).length;
    const candidates = module.functions.filter(f => evidence(f, threshold).rank <= 1).length;
    const invocations = module.functions.reduce((total, f) => total + f.invocations, 0);
    const collapsed = collapsedModules.has(moduleId);
    const summary = `${nf.format(observed)} of ${nf.format(module.functions.length)} functions observed · ${nf.format(candidates)} candidates · ${nf.format(invocations)} invocations`;
    const statuses = [];
    if (!module.functions.some(external)) statuses.push(`<span class="badge unused-internal">No externally callable functions</span>`);
    if (observed === 0) {
      statuses.push(`<span class="badge unused-external">Entire module unobserved</span>`);
    } else if (moduleEvidence(module).rarelyObserved) {
      statuses.push(`<span class="badge rare-external">Rarely observed</span>`);
    }
    const moduleStatus = statuses.join("");
    const header = `<tr class="module-row"><td colspan="8"><button class="module-toggle" data-module="${esc(moduleId)}" aria-expanded="${!collapsed}"><span class="module-chevron">${collapsed?"▸":"▾"}</span><code title="${esc(module.id)}">${esc(module.displayId)}</code>${moduleStatus}<span class="module-summary">${esc(summary)}</span></button></td></tr>`;
    return header + (collapsed ? "" : visibleFunctions.map(functionRow).join(""));
  }).join("") : `<tr><td colspan="8" class="empty">No functions match these filters.</td></tr>`;
  for (const button of document.querySelectorAll("button[data-module]")) button.addEventListener("click", () => {
    const moduleId = button.dataset.module;
    if (collapsedModules.has(moduleId)) collapsedModules.delete(moduleId); else collapsedModules.add(moduleId);
    renderFunctions();
  });
  for (const button of document.querySelectorAll("button[data-function]")) button.addEventListener("click", () => renderDetails(functions.get(button.dataset.function), button));
}

let detailAnchor = null;
function closeDetails() {
  document.getElementById("details").hidden=true;
  detailAnchor=null;
}

function positionDetails(anchor) {
  const details=document.getElementById("details");
  if (details.hidden || !anchor?.isConnected) return closeDetails();
  const margin=10, gap=8, anchorRect=anchor.getBoundingClientRect();
  details.style.visibility="hidden";
  details.style.left=`${margin}px`;
  details.style.top=`${margin}px`;
  const detailRect=details.getBoundingClientRect();
  const left=Math.min(Math.max(margin,anchorRect.left),window.innerWidth-detailRect.width-margin);
  const below=anchorRect.bottom+gap;
  const above=anchorRect.top-detailRect.height-gap;
  const top=below+detailRect.height<=window.innerHeight-margin ? below : Math.max(margin,above);
  details.style.left=`${left}px`;
  details.style.top=`${top}px`;
  details.style.visibility="visible";
}

function renderDetails(f, anchor) {
  const details=document.getElementById("details");
  detailAnchor=anchor;
  details.hidden=false;
  document.getElementById("detail-title").innerHTML=`<code title="${esc(f.id)}">${esc(f.displayId)}</code>`;
  const summary=[
    ["Visibility",`${f.visibility}${f.is_entry?" · entry":""}${f.is_native?" · native":""}`],
    ["Transactions",nf.format(f.transactions)],
    ["Invocations",nf.format(f.invocations)],
    ["Retained calling entries",nf.format(new Set(f.roots.map(r=>rawFunctionName(r.root_function))).size)],
    ["Retained immediate caller addresses",nf.format(f.callers.size)]
  ];
  document.getElementById("detail-summary").innerHTML=summary.map(([label,value])=>`<div><span class="meta">${esc(label)}</span><span class="value">${esc(value)}</span></div>`).join("");
  const roots=[...f.roots].sort((a,b)=>b.invocation_count-a.invocation_count || functionName(a.root_function).localeCompare(functionName(b.root_function)));
  let missingRoots;
  if (f.invocations === 0) {
    missingRoots="No calls to this function were observed in the replay range.";
  } else if ((report.schema_version ?? 0) < 5) {
    missingRoots="Calling-entry attribution was not retained by this report version. Run the updated workflow to collect it.";
  } else {
    missingRoots="Calling-entry attribution was not retained for this function because its per-function limit was reached.";
  }
  document.getElementById("detail-roots").innerHTML=roots.length?`<table><thead><tr><th>Entry function</th><th>Outcome</th><th class="number">Transactions</th><th class="number">Invocations</th><th>Versions</th></tr></thead><tbody>${roots.map(r=>`<tr><td><code title="${esc(rawFunctionName(r.root_function))}">${esc(r.root_function ? functionName(r.root_function) : "<unknown entry>")}</code></td><td>${esc(r.outcome)}</td><td class="number">${nf.format(r.transaction_count)}</td><td class="number">${nf.format(r.invocation_count)}</td><td>${nf.format(r.first_version)}–${nf.format(r.last_version)}</td></tr>`).join("")}</tbody></table>`:`<div class="empty">${esc(missingRoots)}</div>`;
  const paths=[...f.paths].sort((a,b)=>b.invocation_count-a.invocation_count);
  const missingPaths=f.invocations === 0 ? "No calls to this function were observed in the replay range." : report.usage_detail_truncated ? "Immediate call-path detail was not retained for this function because the report reached its global detail limit." : "Immediate call-path detail is unavailable for this observed function.";
  document.getElementById("detail-paths").innerHTML=paths.length?`<table><thead><tr><th>Immediate caller</th><th>Root payload</th><th>Kind</th><th>Outcome</th><th class="number">Transactions</th><th class="number">Invocations</th><th>Versions</th></tr></thead><tbody>${paths.map(p=>`<tr><td><code>${esc(functionName(p.caller))}</code></td><td><code>${esc(functionName(p.root_function))}</code></td><td>${esc(p.call_kind)}</td><td>${esc(p.outcome)}</td><td class="number">${nf.format(p.transaction_count)}</td><td class="number">${nf.format(p.invocation_count)}</td><td>${nf.format(p.first_version)}–${nf.format(p.last_version)}</td></tr>`).join("")}</tbody></table>`:`<div class="empty">${esc(missingPaths)}</div>`;
  positionDetails(anchor);
}

document.getElementById("detail-close").addEventListener("click",closeDetails);
document.addEventListener("keydown",event=>{if(event.key==="Escape")closeDetails();});
document.addEventListener("pointerdown",event=>{
  const details=document.getElementById("details");
  if (!details.hidden && !details.contains(event.target) && !event.target.closest("button[data-function]")) closeDetails();
});
document.addEventListener("scroll",event=>{
  if (!document.getElementById("details").contains(event.target)) positionDetails(detailAnchor);
},true);
window.addEventListener("resize",()=>positionDetails(detailAnchor));

for (const id of ["search","class-filter","visibility","threshold"]) document.getElementById(id).addEventListener(id==="search"||id==="threshold"?"input":"change",renderFunctions);
document.getElementById("toggle-entrypoint-callers").addEventListener("click", () => {
  const expand = visibleEntrypointAddresses.length > 0 && visibleEntrypointAddresses.every(address => expandedEntrypointAddresses.has(address));
  for (const address of visibleEntrypointAddresses) {
    if (expand) expandedEntrypointAddresses.delete(address); else expandedEntrypointAddresses.add(address);
  }
  renderActiveEntryFunctionCallers();
});
document.getElementById("toggle-modules").addEventListener("click", () => {
  const expand = visibleModuleIds.length > 0 && visibleModuleIds.every(moduleId => collapsedModules.has(moduleId));
  for (const moduleId of visibleModuleIds) {
    if (expand) collapsedModules.delete(moduleId); else collapsedModules.add(moduleId);
  }
  renderFunctions();
});
renderCards();
renderActiveEntryFunctionCallers();
renderFunctions();
</script>
</body>
</html>
"#;
