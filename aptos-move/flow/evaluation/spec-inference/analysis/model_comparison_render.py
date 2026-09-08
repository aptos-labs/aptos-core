"""Render the recorded UC comparison as Markdown, including provenance caveats."""
import json
from analysis.model_comparison import OUT

def table(headers, rows):
    return '| '+' | '.join(headers)+' |\n| '+' | '.join(['---']*len(headers))+' |\n'+''.join('| '+' | '.join(map(str,r))+' |\n' for r in rows)+'\n'
def n(v):return f'{v:,.0f}'
def money(v):return f'${v:,.3f}'
def label(r):return r['model'].upper()+' '+('agent' if r['arm']=='agent_only' else 'guided')
def render():
    data=json.loads((OUT/'analysis.json').read_text());rows=data['rows'];groups=data['groups'];ci=data['bootstrap_intervals']
    text='''# GLM–Opus comparison: UC-credits-008

**Provenance error: this experiment did not include `wrwg/inf-tool-fixes` commit `77f837d620`.** Both models ran on `950e413e46` plus the later working-tree WP/harness fixes. These results describe that recorded apparatus; they do **not** establish performance after applying the intended `inf-tool-fixes` branch. This omission was discovered after execution, during analysis.

All 16 sessions finished on 2026-09-07. Opus achieved 8/8 operational and strict successes; GLM achieved 6/8. Every successful session killed all three essential held-out mutants. On this task, guided WP reduced GLM's observed mean output substantially, while Opus's two workflows used almost identical mean output. The small sample, two GLM failures, and omitted fixes limit the interpretation.

## 1. What actually ran

The task is a tier lookup returning the three fields of the **last** qualifying tier, with zero defaults. Four fresh replicates ran for each model/workflow combination: agent-only and hybrid-guided. Flexible hybrid was not included.

The recorded initial package, arm-specific plugin manifests, harness, binary, and held-out mutant digests match across models. Both configurations use effort `max`, Claude Code 2.1.258, Agent SDK 0.2.139, a nominal 150,000-output-token budget, 3,600-second session deadline, and 40-second prover timeout. GLM ran first, then Opus, each at concurrency 3. Model conditions were not interleaved. The full launch took about 63.6 minutes; summed session times are larger because sessions overlap.

Opus used subscription OAuth; the launcher removes `ANTHROPIC_API_KEY` and does not fall back to it. GLM used the existing Z.ai profile. Configured/native model labels are `glm-5.3[1m]` (native responses also say `glm-5.3`) and `claude-opus-5`.

### Missing upstream fixes

The local reflogs show:

- 06:42:28 UTC: `wrwg/corpus3.2-run1` was created from `wrwg/inf-tool-fixes`, then pointing to `950e413e46`.
- 06:57:57 UTC: `wrwg/inf-tool-fixes` advanced to `77f837d620` (`Fix WP abort paths and qualified filters`).
- The corpus branch remained at `950e413e46`. The comparison binary was built there with the archived working-tree patch.

Missing changes include address-qualified filter resolution, skipping call summaries on unconditional-abort continuations, more detailed partial-abort causes, and guidance on abstract versus concrete abort codes. Our later partial-abort diagnostic changes overlap part of that work but do not incorporate the commit.

There is direct evidence of a consequence: GLM guided r01's first WP call used `decibel_campaign::extracted_tier_lookup::credits_for_duration_days` and failed with `no module matching decibel_campaign::extracted_tier_lookup`. Retrying the short module/function form worked. This is exactly the area the omitted qualified-filter fix addresses. We have not established that the missing abort-path changes affected this task. The commit does not add a scratch-file deletion tool or document tuple-return names, so it should not be claimed to fix those separate problems.

No outcomes were rewritten or replayed to hide the omission. The intended corrected-apparatus experiment remains unperformed.

## 2. Outcomes and resource use

A strict success requires operational acceptance and killing every essential held-out mutant. Failures remain in all four-run resource averages.

'''
    text+=table(['Condition','Strict success','Mean output','Median output','Mean minutes','SDK estimate total'],[[label(g),f"{g['strict_successes']}/4",n(g['output_tokens']['mean']),n(g['output_tokens']['median']),f"{g['wall_seconds']['mean']/60:.2f}",money(g['sdk_estimated_cost_usd']['total'])] for g in groups])
    ga,gh,oa,oh=groups
    text+=f'''The guided/agent mean-output ratio is **{gh['output_tokens']['mean']/ga['output_tokens']['mean']:.3f} for GLM** ({100*(1-gh['output_tokens']['mean']/ga['output_tokens']['mean']):.1f}% lower output) and **{oh['output_tokens']['mean']/oa['output_tokens']['mean']:.3f} for Opus** ({100*(1-oh['output_tokens']['mean']/oa['output_tokens']['mean']):.2f}% lower). Opus guided has a {100*(oh['sdk_estimated_cost_usd']['total']/oa['sdk_estimated_cost_usd']['total']-1):.1f}% higher SDK dollar estimate despite nearly equal output, reflecting input/cache use.

GLM's agent-only mean is dominated by r01: 180,278 output tokens and failure after 53.6 minutes. The successful GLM agent-only runs averaged 25,744 output tokens; successful guided runs averaged 15,229. That selected-success comparison also favors guided, but excludes different failures and is not an unbiased treatment estimate.

For a budget-normalized sensitivity analysis, assign each failed session the declared cap (150,000 output tokens or 60 minutes) as its restricted cost to success. This is a scoring convention, not a replacement for actual usage:

'''
    text+=table(['Condition','Restricted output to success, mean','Restricted minutes to success, mean'],[[label(g),n(g['restricted_output_to_success']['mean']),f"{g['restricted_wall_to_success']['mean']/60:.2f}"] for g in groups])
    text+='''### Token and cost totals

“Fresh input” excludes cache creation and cache reads. Thinking is a subset of output, not extra tokens to add. GLM reports zero thinking tokens; that is not evidence of zero reasoning. The two providers' reported thinking counters are not comparable.

'''
    totals=[]
    for model in ['glm','opus','all']:
        rs=[r for r in rows if model=='all' or r['model']==model]
        totals.append([model.upper(),*[n(sum(r[k] for r in rs)) for k in ['input_tokens','cache_creation_input_tokens','cache_read_input_tokens','output_tokens','reported_thinking_tokens']],money(sum(r['sdk_estimated_cost_usd'] for r in rs))])
    text+=table(['Model','Fresh input','Cache creation','Cache reads','Output','Reported thinking','SDK estimate'],totals)
    text+='''**Dollar figures are SDK estimates, not invoices.** Opus reports `costBasis: list`; GLM reports `costBasis: unknown`. All 16 estimates reconcile arithmetically with $5/M fresh input, $0.50/M cache reads, and $25/M output, with $10/M for Opus's one-hour cache writes. For GLM this does not establish the actual Z.ai tariff: the apparent cost comparison uses the SDK's same token weights, not verified provider billing. Actual subscription charges and actual GLM invoice amounts are unavailable. The combined SDK estimate is $31.6078495.

Usage comes from the sum of per-query native `usage` records. Session-cumulative API time and cost are taken once per SDK session, avoiding repeated counting of earlier controller turns. All 16 usage summaries are complete and output totals reconcile with controller totals; there are no usage/cost reconciliation findings. Local tool time is the sum of Flow MCP `tool_end.duration_us`; it excludes judge-side work and is not total CPU time.

'''
    text+=table(['Condition','API seconds total','MCP seconds total','WP seconds total','Tool calls','Edit/Write calls','Verifier/check calls','WP calls'],[[label(g),f"{g['api_seconds']['total']:.1f}",f"{g['local_mcp_seconds']['total']:.1f}",f"{g['wp_seconds']['total']:.2f}",n(g['tool_calls']['total']),n(g['edits']['total']),n(g['verifier_calls']['total']),n(g['wp_calls']['total'])] for g in groups])
    text+='''## 3. Uncertainty

This is one deliberately selected task, chosen because an earlier comparison showed a strong reversal. It is exploratory and cannot support a corpus-wide claim. Tokenizers, provider behavior, context/cache handling, and sequential model order also differ.

The following are descriptive 95% percentile bootstrap intervals from 10,000 resamples, seed 20260907. Condition means resample four sessions. Workflow contrasts resample the four replicate blocks within each model, preserving the two workflows together; the two models are resampled independently. Four repetitions provide weak information about rare long runs. No per-session confidence interval is assigned to directly observed usage, and no naive binomial interval treats this single task as eight independent tasks.

'''
    text+=table(['Condition','Mean output [95% interval]','Mean SDK estimate [95% interval]'],[[label(g),f"{n(g['output_tokens']['mean'])} [{n(g['output_tokens']['low95'])}, {n(g['output_tokens']['high95'])}]",f"{money(g['sdk_estimated_cost_usd']['mean'])} [{money(g['sdk_estimated_cost_usd']['low95'])}, {money(g['sdk_estimated_cost_usd']['high95'])}]"] for g in groups])
    ratios=[gh['output_tokens']['mean']/ga['output_tokens']['mean'],oh['output_tokens']['mean']/oa['output_tokens']['mean']]
    text+=table(['Contrast','Estimate','95% interval'],[[name,f'{estimate:.3f}',f"[{ci[key]['low95']:.3f}, {ci[key]['high95']:.3f}]"] for name,estimate,key in [('GLM guided / agent output',ratios[0],'glm_guided_agent_ratio'),('Opus guided / agent output',ratios[1],'opus_guided_agent_ratio'),('Opus ratio / GLM ratio',ratios[1]/ratios[0],'ratio_of_ratios_opus_over_glm')]])
    text+='''All ratio intervals include 1. The observed model-by-workflow pattern is compatible with WP helping GLM more, but this experiment does not establish that interaction precisely. The omitted branch fixes further limit any intended post-fix conclusion.

## 4. What the agents did

**Shared solution.** Successful sessions characterized the last qualifying tier using a recursive prefix helper and loop invariants aligned with one iteration. Some helpers returned the three fields, others a tier or last-match index plus selectors. The accepted contracts constrain all three returns and state totality with `aborts_if false`.

**GLM agent-only spent substantial effort discovering language conventions.** Self-reports from r02–r04 describe probing `result`, `result_0`, and then discovering the correct `result_1/result_2/result_3` names. Logged name-resolution diagnostics corroborate this account. GLM r02 succeeded after 49,179 output tokens. Across all four agent-only runs GLM made 95 Edit/Write calls and 33 verifier/check calls, compared with Opus's 9 and 9. These counts include failures and exploratory probes; they are not all necessary repairs.

**GLM agent-only r01 did not produce an accepted contract.** It exhausted two 61-turn SDK queries and logged 180,278 output tokens. After the first query, one of three controller refutations survived, leaving normal-result behavior insufficiently constrained. Later checks rejected implementation changes. Its final self-report was empty, so the explanation relies on controller and tool logs, not an inferred account of the agent's intentions.

**GLM guided usually used bounded WP evidence to choose prefix helpers.** All four sessions encountered the expected missing-invariant diagnostic. Several self-reports describe recognizing the last-match semantics from the bounded facts and rerunning WP after adding invariants. Guided r01 also hit the missing qualified-filter fix; guided r02 used a scratch module to probe tuple-return conventions.

**Opus used the language and helper pattern with less exploration.** Every guided Opus run made exactly two WP calls: an initial missing-invariant response, then successful inference after invariants. Reports describe simplifying generated quantified or initialization-guard clauses into readable case/selector contracts. Direct Opus runs obtained the same semantic result without WP tool calls. Their references to “WP reading” mean reasoning about the code, not access to the unavailable tool. Two direct runs deliberately tested a weakened boundary condition and observed invariant failures; these logged failures are useful soundness probes, not accidental repair churn.

**No WP performance recurrence was observed on this task.** The 17 logged WP calls all finished in under 0.8 seconds. Opus's four initial WP errors and GLM's corresponding missing-invariant responses are expected workflow feedback. GLM additionally had a qualified-filter RPC error and a probe-related tool error. This does not validate every fix on other corpus tasks.

## 5. Failures and apparatus limitations

1. **Missing `inf-tool-fixes`.** The branch ancestry was not checked against the intended parent before building. A recorded commit and binary hash make the executed apparatus identifiable, but do not prove it includes the intended changes. Future preflight must explicitly require the relevant ancestor/patch and test its behavior. Preserve this round as a diagnostic comparison; a corrected comparison needs a new recorded revision.
2. **The output limit is not a streaming hard cap.** The controller checks accumulated output only after the SDK query returns and the candidate is judged. GLM r01 overshot 150,000 by 30,278 tokens (20.2%) before termination. Actual usage remains 180,278 in the tables; it has not been clipped. Enforcing a hard aggregate cap needs in-query accounting/cancellation and explicit incomplete-usage handling.
3. **Scratch-file cleanup was unavailable.** GLM guided r02's final rejection named added `sources/etna/zz_probe.move` and `.ignore`. The agent reported that allowed tools could not delete them; emptying them did not satisfy the file-set policy. A target verification success is present in the logs, but the session remains a failure and was not given held-out credit. A narrowly scoped deletion operation for agent-created workspace files is a separate fix to consider.
4. **Repair counts need interpretation.** Some compiler failures were deliberate naming probes and two Opus invariant failures were intentional negative checks. Counts alone do not establish confusion or unsoundness.

## 6. Held-out strength checks

Scoring used the precommitted, as-scheduled held-out set after all model sessions ended, at the same 40-second prover limit and concurrency 1 per scoring round. The three essential mutations were a strict duration boundary, returning rank from credits, and a nonzero default credit value. All 14 operational successes killed all three: **42/42 evaluated mutations killed**, with no survivors or inconclusive outcomes. The two failed GLM sessions were not scored and remain strict failures.

This shows equivalent discrimination on this three-mutant set among accepted contracts. It is not a proof that all accepted contracts have equal strength against arbitrary defects.

## 7. Every session

Input includes fresh input, cache creation, and cache reads. The companion CSV separates those categories for every session. Dollar figures retain the SDK-estimate caveat above.

'''
    text+=table(['Model','Workflow','Rep','Outcome','Total input','Output','Minutes','SDK estimate','Held-out'],[[r['model'].upper(),'agent' if r['arm']=='agent_only' else 'guided',r['replicate'],{'operational_success':'success','output_token_budget_exhausted':'output budget','repeated_forbidden_weakening':'scratch files'}[r['terminal_status']],n(r['total_input_tokens']),n(r['output_tokens']),f"{r['wall_seconds']/60:.2f}",money(r['sdk_estimated_cost_usd']),f"{r['killed']}/{r['essential_mutants']}" if r['essential_mutants'] is not None else 'not scored'] for r in rows])
    text+='''The most defensible reading is that this recorded apparatus exposes a large GLM search tail on UC-credits, while Opus solves it consistently with either workflow. Guided WP reduces the observed GLM resource tail but does not improve its success count in these four repetitions. The intended comparison including `inf-tool-fixes` still needs to be run before drawing conclusions about that revision.

Evidence: [per-session CSV](per-session.csv), [metrics and intervals](analysis.json), [self-reports and tool sequences](session-details.json), [launch plan](plan.md), [GLM mutation summary](../corpus3.2-model-compare-uc-glm/mutation-summary.json), [Opus mutation summary](../corpus3.2-model-compare-uc-opus/mutation-summary.json), and [recorded apparatus](../corpus3.2-model-compare-uc-glm/apparatus.json). Full transcripts and candidate diagnostics remain in each model round's `runs/` directory. Artifacts are local; no proprietary source is reproduced in this report.

Reproduce the tables with `.venv/bin/python -m analysis.model_comparison`, then `.venv/bin/python -m analysis.model_comparison_render`. These commands analyze recorded results; they do not call models or rerun evaluation. The narrative includes manual log review and the subsequent branch-provenance audit.
'''
    (OUT/'analysis.md').write_text(text)
    print(OUT/'analysis.md')
if __name__=='__main__':render()
