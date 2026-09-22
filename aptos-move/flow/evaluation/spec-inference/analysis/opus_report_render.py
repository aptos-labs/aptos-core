"""Human-readable companion to opus_report's auditable datasets."""
import collections, html, json, statistics
from analysis.opus_report import table

def render(root,out,data,rows,details):
    done=[r for r in rows if 'sdk_cost_usd' in r];t=data['totals'];arms=data['arms']
    def interval(c):return f"{c['mean']:,.2f} [{c['low95']:,.2f}, {c['high95']:,.2f}]"
    partial=sum(r['recomputed_cost_usd'] for r in data['interrupted'])
    tools=sum((collections.Counter(d['tools']) for d in details),collections.Counter())
    missing_tasks=sorted({r['task_id'] for r in rows}-{r['task_id'] for r in done})
    report=f'''# Corpus 3.2 run 1 — Opus execution report

Generated 2026-09-07 from the archived local run. **Partial round; execution stopped.**

## Main findings

- **135 / 240 scheduled sessions have recorded outcomes:** 133 operational successes and two infrastructure failures. There are 45 complete three-arm replicate blocks across 19 of 20 tasks. No recorded outcomes for {', '.join(missing_tasks)}. The remaining 105 cells are marked `batch_aborted`; that status does not distinguish never-started work from work killed in flight.
- Recorded sessions used **{t['total_input_tokens']:,} input tokens**, including cache writes and reads, and **{t['output_tokens']:,} output tokens**. Their **${t['sdk_cost_usd']:.2f} SDK estimate** reconciles with token pricing for every recorded session.
- The interrupted concurrency-four startup adds at least **${partial:.4f}** in identifiable token usage, making the observed API-equivalent lower bound **${t['sdk_cost_usd']+partial:.4f}**. Missing terminal usage and final-abort in-flight traces prevent a complete launch-wide spend total.
- Flexible hybrid used WP in all 45 recorded sessions. Its task-weighted cost was $0.243 higher per session than agent-only (95% task-cluster CI $0.138–$0.352). Guided versus flexible cost difference was −$0.013 (−$0.122–$0.091); these data do not establish a difference between those two workflows.
- The agents often supplied invariants, simplified generated contracts, and tested clauses with intentional counterexamples. Controller refutation exposed insufficient partition contracts despite initial acceptance. **Held-out mutation scoring has not run**, so operational success is not evidence of strict adequacy.

## Scope, provenance, and cost meaning

Model: `claude-opus-5`, Claude subscription OAuth through the Anthropic Agent SDK (0.2.139; Claude Code 2.1.258). Configuration: four replicates, three arms, 20 tasks, concurrency three, 150,000 output-token budget and 3,600-second wall budget per session; solver budget 40 seconds. Actual recorded maximum wall time was {max(r['wall_seconds'] for r in done):,.1f} seconds. No model-budget failures were recorded. Resource figures include fresh SDK sessions created by infrastructure retries.

Base source commit: `950e413e46090d2056740c36dd7a77b1764b6936`, plus the archived apparatus patch. Consult [schedule provenance](../schedule/pilot-manifest.json), [configuration](../config.json), [apparatus patch](../apparatus-source.patch), and [launch result](../launch-report.json) for the authoritative hashes and settings. This report reads artifacts; it does not rerun agents or proofs.

**Dollar figures are API-equivalent estimates, not subscription invoice charges.** The telemetry has contradictory overage fields (`isUsingOverage: false` together with `overageInUse: true` and `overageStatus: allowed`). It cannot establish what the account was billed. No account invoice or hidden HTTP headers were available.

[Official Anthropic pricing](https://platform.claude.com/docs/en/about-claude/pricing) was consulted on 2026-09-07 and archived in [pricing.json](pricing.json): per million tokens, fresh input $5, one-hour cache writes $10, five-minute writes $6.25, cache reads $0.50, output $25. All recorded cache writes have an explicit duration. Thinking is a subset of output, not another billable category.

## Exact observed totals

'''
    costs=[('Fresh input','input_tokens',5),('Cache creation','cache_creation_input_tokens',10),('Cache reads','cache_read_input_tokens',.5),('Output, including thinking','output_tokens',25)]
    report+=table(['Category','Tokens','API-equivalent dollars'],[(name,f"{t[k]:,}",f"${t[k]*rate/1e6:,.6f}") for name,k,rate in costs])
    report+=f'''Total input processed: **{t['total_input_tokens']:,}**. Input plus output: **{t['total_tokens']:,}**. Reported thinking: **{t['thinking_tokens']:,}** ({100*t['thinking_tokens']/t['output_tokens']:.1f}% of output); this counts only the reported thinking fields. Cache reads account for {100*t['cache_read_input_tokens']/t['total_input_tokens']:.1f}% of input volume.

The 135 evaluation sessions contain **{t['sdk_sessions']} distinct SDK sessions and {t['sdk_queries']} completed queries**, including four infrastructure restarts. Summed controller wall time is **{t['wall_seconds']/3600:.2f} session-hours**, and summed SDK API duration is **{t['api_seconds']/3600:.2f} hours**. These are summed resource times, not elapsed batch time; concurrent sessions overlap. SDK API duration is provider-reported and does not provide a clean CPU/prover time decomposition.

Exact totals and individual recorded session values do not need sampling confidence intervals. Their limitation is missing telemetry, which a statistical interval cannot repair. The ordinary session-weighted mean cost is ${t['sdk_cost_usd']/len(done):.4f}; median ${statistics.median(r['sdk_cost_usd'] for r in done):.4f}; range ${min(r['sdk_cost_usd'] for r in done):.4f}–${max(r['sdk_cost_usd'] for r in done):.4f}.

## Per-arm totals and confidence intervals

A = agent-only; H-F = flexible hybrid; H-G = guided hybrid. Means below give each observed task equal weight, then average its observed replicates. Unequal replicate coverage makes these differ from total / sessions.

'''
    report+=table(['Arm','Recorded / success','Cost total','Input total','Output total','Mean cost [95% CI]','Mean wall seconds [95% CI]'],[(a,f"{v['sessions']} / {v['successes']}",f"${v['totals']['sdk_cost_usd']:.4f}",f"{v['totals']['total_input_tokens']:,}",f"{v['totals']['output_tokens']:,}",interval(v['mean_ci']['sdk_cost_usd']),interval(v['mean_ci']['wall_seconds'])) for a,v in arms.items()])
    report+=table(['Arm','Mean input [95% CI]','Mean output [95% CI]'],[(a,interval(v['mean_ci']['total_input_tokens']),interval(v['mean_ci']['output_tokens'])) for a,v in arms.items()])
    report+='Across all arms, task-weighted mean cost is $'+interval(data['all_mean_ci']['sdk_cost_usd'])+'; mean output tokens '+interval(data['all_mean_ci']['output_tokens'])+'.\n\n'
    report+='### Paired arm contrasts\n\nAll contrasts use the same 45 complete task–replicate blocks across 19 tasks, retaining within-task dependence. Positive means the first arm uses more resources.\n\n'
    report+=table(['Contrast','Cost difference [95% CI]','Output difference [95% CI]','Wall seconds difference [95% CI]'],[(k+' '+v['definition'],interval(v['metrics']['sdk_cost_usd']),interval(v['metrics']['output_tokens']),interval(v['metrics']['wall_seconds'])) for k,v in data['contrasts'].items()])
    report+='''Intervals use 10,000 percentile bootstrap resamples of tasks, seed 20260907. Replicates and arms stay together within a sampled task. These are **exploratory intervals conditional on the recorded subset**: outcome-dependent stopping can bias the subset, and bootstrap resampling cannot recover the missing task or missing replicates. They do not establish a confirmatory treatment effect for the full corpus.

For the planned C1/C2 family, the machine-readable results also provide 97.5% intervals (Bonferroni across two contrasts for each metric, not across all metrics). C1 cost: $0.124–$0.368; C2 cost: −$0.138–$0.105. No confirmatory p-values are claimed. The observed success proportions are descriptive: an all-success bootstrap can give a degenerate [100%,100%] interval, which does not establish certainty. Strict success is unmeasured; the core-mode `strict_success: false` placeholder is not a scored model failure. Restricted time-to-success estimates under budget censoring are not presented as primary results because the two nonsuccesses were invalid infrastructure outcomes, and the round is incomplete.

## What the agents actually did

'''
    report+=table(['Arm','WP-using sessions','WP calls','WP before first explicit Edit/Write','Sessions with verifier failures'],[(a,f"{v['wp_sessions']}/45",v['totals']['wp_calls'],v['wp_before_edit'],v['verifier_failure_sessions']) for a,v in arms.items()])
    report+='''All 90 hybrid sessions used WP, while agent-only made zero WP calls. Guided sessions always called WP before their first explicit file edit; flexible sessions did so in 34/45 cases, with 11 editing first. No hybrid session postponed its first WP call until after its first verification call. Ordering is measured across the full evaluation session, including retries. WP can itself inject specs, so “explicit edit” refers only to Edit/Write tools, not all file mutations. These observations support early WP adoption; they do not by themselves show that generated clauses were retained.

'''
    report+=table(['Logged tool','Calls'],[(k,v) for k,v in tools.most_common()])
    report+='''The logs contain 445 targeted Edit calls and 16 Write calls. Read/Grep/Glob dominate exploration. The 364 verification/check calls include 68 tool-error results across 52 sessions; one additional package-status compiler failure appears in the miner. Diagnostic labels include 30 postcondition failures, 12 uncovered aborts, four solver timeouts, three loop-invariant base failures and two induction failures. These are automatic diagnostic labels, not a count of agent mistakes: deliberate negative tests also produce verification failures, and some infrastructure/query errors lie outside this verifier taxonomy.

In the final self-reports, 78 mention invariants, 40 recursion, 56 “partial”, 37 “sathard”, 29 probes and 36 counterexamples. These are literal keyword counts, not validated strategy labels. The full reports, including earlier reports before repair or retry, are preserved in the interactive report and session-details.json.

### Evidence-checked examples

1. **QP-part-025, r02, agent-only — controller feedback caused a substantive strengthening.** The agent says its initial contract constrained only partition shape and that it added an exact vector post-state using recursive helpers. Controller refutation moved from **3/4 to 4/4**. The transcript records an intentional edit replacing the final swap with `values[store] = p` (sequence 1147), a postcondition counterexample (1158), and restoration of the swap (1185). This corroborates the reported negative test. Final implementation equality was checked by the judge. The claim that no behaviorally different implementation could satisfy the contract is broader than the finite refutation evidence.
2. **QP-part-025, r03, guided — acceptance did not guarantee adequate constraints.** The controller recorded **3/4, 3/4, then 4/4**, requiring two strengthening rounds. The final report describes replacing shape-only assertions with an exact loop-aligned recursive model. This is a concrete example of refutation adding value beyond the agent's acceptance self-report.
3. **QP-part-025, r03, flexible — the most expensive session ($4.6190) includes a retry.** It used two SDK sessions; an initial refutation was inconclusive before the retry converged to 4/4. The final report says the final recursive model verified “first try,” but the whole evaluation transcript also contains an earlier invariant-base failure and a repair adding the lower bound `0 <= j`, plus intentional failing postcondition probes. “First try” therefore describes a local attempt, not the entire recorded session. Its claim about WP double-applying a swap is an agent diagnosis; this report does not independently establish a WP implementation defect.
4. **SM-select-022, r04, guided — a correct candidate check still led to infrastructure failure.** WP logs explicitly warn about incomplete abort characterization after a memory-havocking loop. The agent edits the partial generated specification and reports an exact abort model. The log also shows an `old(start)` context error, its repair, and another WP run. Candidate checking accepted the final tree, but controller refutation twice reached only **2/3**, with the draw-count-off-by-one mutant timing out. Its final “complete” self-report does not override that unresolved controller outcome.

The detailed session cards link directly to each controller log and agent transcript; their sequence numbers identify the cited actions. Claims about proof completeness, tool defects, or redundancy remain claims unless corroborated by the controller or specific tool evidence. No hidden reasoning is reconstructed from token counts.

## Why execution stopped, and what remains unknown

There were four infrastructure restarts: QP-part-025 r03 flexible and SM-select-022 r02 agent-only recovered; SM-select-022 r01 agent-only and r04 guided did not. Six refutation passes were inconclusive across those sessions. Separately, three partition sessions required four total contract-strengthening rounds after surviving refutation mutants.

The two terminal infrastructure failures reached the configured abort threshold. Both repeatedly timed out on `SM-select-022-draw-count-off-by-one`; neither timeout is evidence that the mutant was killed or survived. The batch stopped at approximately 11:22 UTC on September 7, 2026. Of 240 scheduled sessions, 105 have no published outcome. The dispatcher marks both queued and in-flight aborts alike, and sandbox cleanup can remove their staging traces. They are excluded from means, not counted as model failures or assigned zero cost.

The four earlier startup sessions have no terminal SDK result. Their deduplicated assistant-message usage gives only a lower bound; unfinished output/thinking can be absent. The report does not project the remainder's cost from this outcome-dependent subset or infer subscription charges.

## Audit, reproducibility, and files

- [per-session.csv](per-session.csv): all 240 scheduled evaluation sessions; missing observations have blank metrics, not zero. Token categories, cost, wall/API time, tool counts, WP order, refutation counts and outcome are included.
- [per-sdk-session.csv](per-sdk-session.csv): 139 distinct recorded SDK sessions; retries remain separately visible.
- [per-query.csv](per-query.csv): 143 query records including usage, reported latency, receipt timing, turns, stop reasons and errors. **Its `total_cost_usd` and `duration_api_ms` fields are cumulative SDK-session counters and must not be summed across queries.**
- [session-details.json](session-details.json): all query self-reports and logged tool ordering for each recorded evaluation session.
- [analysis.json](analysis.json): exact totals, arm/contrast intervals, rate-limit flags, partial-start estimates and reconciliation findings.
- [interrupted-startup.csv](interrupted-startup.csv): four incomplete startup traces, separate from the 240-session scheduled table.

Token accounting sums per-query usage once; cumulative SDK cost uses only the last result per SDK session, then sums sessions. Nested iteration usage and thinking are not added again. Price recomputation matches every recorded session within $0.00001; the controller output-token totals agree. Complete raw logs remain authoritative. Timings do not expose every internal SDK/network or prover stage, and unobserved work cannot be reconstructed from final results.

Rebuild from the evaluation directory:

```sh
.venv/bin/python -m analysis.opus_report --round-dir evaluation-artifacts/corpus3.2-run1-opus-metrics
```

This report contains private task identifiers and agent self-reports; it remains in the local artifact directory.

## Per-task recorded totals

'''
    taskrows=[]
    for task in sorted({r['task_id'] for r in rows}):
        rr=[r for r in done if r['task_id']==task];taskrows.append([task,len(rr),sum(r['success'] for r in rr),f"{sum(r['total_input_tokens'] for r in rr):,}" if rr else 'unobserved',f"{sum(r['output_tokens'] for r in rr):,}" if rr else 'unobserved',f"${sum(r['sdk_cost_usd'] for r in rr):.4f}" if rr else 'unobserved'])
    report+=table(['Task','Recorded / 12','Success','Input','Output','API-equivalent cost'],taskrows)
    (out/'report.md').write_text(report)
    # Standard-library rendering: escaped markdown source retains exact narrative;
    # HTML session tables/cards provide filtering and full evidence inspection.
    heads=['Task','Replicate','Arm','State','Input incl. cache','Output','SDK $','Wall s','WP calls']
    body=[]
    for r in rows:
        vals=[r['task_id'],r['replicate'],r['arm'],r.get('terminal_status',r['dispatch_status']),r.get('total_input_tokens',''),r.get('output_tokens',''),f"{r['sdk_cost_usd']:.6f}" if 'sdk_cost_usd' in r else '',r.get('wall_seconds',''),r.get('wp_calls','')]
        body.append('<tr>'+''.join('<td>'+html.escape(str(v))+'</td>' for v in vals)+'</tr>')
    cards=[]
    for d in details:
        r=d['metrics'];rid=r['run_id'];short=rid.removeprefix(root.name+'-')
        evidence=' '.join(f'<a href="../runs/{html.escape(rid)}/{name}">{name}</a>' for name in ['run.json','judge.json','controller-events.jsonl','claude-events.jsonl','sdk-metrics.json'])
        workflow=f"{r['tool_calls']} tool calls; {r['wp_calls']} WP calls; {r['verifier_calls']} verification/check calls, {r['verifier_failures']} error results; {r['targeted_edits']} Edit and {r['whole_file_rewrites']} Write calls. Refutation: {d['refutation'].get('killed_by_turn',[])}."
        texts=''.join('<h4>SDK '+html.escape(x['session_id'])+' · query '+str(x['query'])+'</h4><pre>'+html.escape(x['text'] or '(No text self-report)')+'</pre>' for x in d['self_reports'])
        cards.append('<details class="card"><summary>'+html.escape(short)+f" · ${r['sdk_cost_usd']:.4f} · "+r['terminal_status']+'</summary><p>'+html.escape(workflow)+'</p><p>'+evidence+'</p>'+texts+'<details><summary>Logged tool order (sequence, timestamp, tool, failure flag)</summary><pre>'+html.escape(json.dumps(d['tool_sequence'],indent=2))+'</pre></details></details>')
    page='''<!doctype html><html lang="en"><meta charset="utf-8"><title>Corpus 3.2 Opus report</title><style>body{font:15px system-ui;margin:2em;color:#17202a}a{color:#0758a0}pre{white-space:pre-wrap;overflow-wrap:anywhere;background:#f5f7fa;padding:1em;line-height:1.5}table{border-collapse:collapse;width:100%}td,th{padding:.45em;border-bottom:1px solid #ddd;text-align:left}th{position:sticky;top:0;background:#e9eef5}input{padding:.7em;width:36em;max-width:90%;margin:1em 0}.card{padding:1em;border:1px solid #ddd;margin:.5em 0}summary{cursor:pointer;font-weight:600}</style><h1>Corpus 3.2 — Opus execution report</h1><p>135 recorded outcomes · 133 operational successes · $167.50 recorded API-equivalent cost. Actual subscription billing is unknown.</p><p><a href="report.md">Markdown report</a> · <a href="per-session.csv">Session CSV</a> · <a href="per-sdk-session.csv">SDK session CSV</a> · <a href="per-query.csv">Query CSV</a> · <a href="analysis.json">Statistics JSON</a></p><details><summary>Full report: findings, confidence intervals, evidence and limitations</summary><pre>'''+html.escape(report)+'''</pre></details><h2>All 240 scheduled sessions</h2><label>Filter task, arm, outcome or report text <input id="filter" placeholder="e.g. QP-part or infrastructure"></label><table><thead><tr>'''+''.join('<th>'+h+'</th>' for h in heads)+'''</tr></thead><tbody>'''+''.join(body)+'''</tbody></table><h2>Self-reports and logged actions</h2>'''+''.join(cards)+'''<script>document.getElementById('filter').addEventListener('input',e=>{const q=e.target.value.toLowerCase();document.querySelectorAll('tbody tr,.card').forEach(x=>x.hidden=!x.textContent.toLowerCase().includes(q));});</script></html>'''
    (out/'report.html').write_text(page)
