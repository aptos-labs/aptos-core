"""Rebuild a read-only, task-clustered report from archived Opus telemetry."""
from __future__ import annotations
import argparse, collections, csv, dataclasses, json, random, statistics
from pathlib import Path
from analysis.round_summary import collect_run
from harness.mine import analyze_run, _tool_calls

FIELDS = ['input_tokens','cache_creation_input_tokens','cache_read_input_tokens','output_tokens']
ARMS = ['agent_only','hybrid_flexible','hybrid_guided']
PRICE = {'input_tokens':5, 'cache_read_input_tokens':.5, 'output_tokens':25, 'cache_write_1h':10, 'cache_write_5m':6.25}

def usage_totals(usages):
    c=collections.Counter()
    for u in usages:
        for k in FIELDS: c[k]+=u.get(k,0) or 0
        c['thinking_tokens']+=(u.get('output_tokens_details') or {}).get('thinking_tokens',0) or 0
        for key, short in [('ephemeral_1h_input_tokens','cache_write_1h'),('ephemeral_5m_input_tokens','cache_write_5m')]:
            c[short]+=(u.get('cache_creation') or {}).get(key,0) or 0
    c['total_input_tokens']=sum(c[k] for k in FIELDS[:3])
    c['total_tokens']=c['total_input_tokens']+c['output_tokens']
    c['recomputed_cost_usd']=sum(c[k]*p/1e6 for k,p in PRICE.items())
    return dict(c)

def dump(p,x): p.write_text(json.dumps(x,indent=2)+'\n')
def csv_write(p,rows):
    keys=list(dict.fromkeys(k for r in rows for k in r))
    with p.open('w') as f:
        w=csv.DictWriter(f,fieldnames=keys);w.writeheader()
        for r in rows:w.writerow({k:json.dumps(v) if isinstance(v,(dict,list)) else v for k,v in r.items()})
def ci(values):
    rng=random.Random(20260907);n=len(values)
    draws=sorted(sum(rng.choices(values,k=n))/n for _ in range(10000))
    return {'mean':statistics.mean(values),'low95':draws[250],'high95':draws[9749], 'low97_5':draws[125],'high97_5':draws[9874],'tasks':n}
def cluster(rows,key):
    groups=collections.defaultdict(list)
    for r in rows:groups[r['task_id']].append(r[key])
    return ci([statistics.mean(v) for v in groups.values()])
def fmt(x):return f'{x:,.2f}' if isinstance(x,float) else f'{x:,}' if isinstance(x,int) else str(x)
def table(headers,rows):return '| '+' | '.join(headers)+' |\n| '+' | '.join(['---']*len(headers))+' |\n'+''.join('| '+' | '.join(map(str,r))+' |\n' for r in rows)+'\n'

def main():
    ap=argparse.ArgumentParser();ap.add_argument('--round-dir',type=Path,required=True);a=ap.parse_args();root=a.round_dir.resolve();out=root/'report';out.mkdir(exist_ok=True)
    launch=json.loads((root/'launch-report.json').read_text());status={r['run_id']:r['status'] for r in launch['results']}
    rows=[];details=[];attempts=[];queries=[];rates=collections.Counter();audit=[]
    for p in sorted((root/'schedule/runs').glob('*.json')):
        spec=json.loads(p.read_text());rid=spec['run_id'];d=root/'runs'/rid
        base={k:spec[k] for k in ['run_id','task_id','arm','replicate']};base['dispatch_status']=status.get(rid,'unknown')
        if not (d/'judge.json').exists():rows.append(base);continue
        record=json.loads((d/'run.json').read_text());s=json.loads((d/'sdk-metrics.json').read_text());summary=collect_run(d);mine=dataclasses.asdict(analyze_run(d));calls=list(_tool_calls(d/'claude-events.jsonl'))
        results=[x['result'] for x in s['results']]
        for i,r in enumerate(results):
            queries.append({**base,'query':i+1,'session_id':r.get('session_id'),**usage_totals([r.get('usage',{})]),**{k:r.get(k) for k in ['duration_ms','duration_api_ms','ttft_ms','ttft_stream_ms','time_to_request_ms','num_turns','stop_reason','terminal_reason','is_error','api_error_status','total_cost_usd']},'receipt_timing':s['query_receipt_timings'][i] if i<len(s['query_receipt_timings']) else None})
        u=usage_totals([r.get('usage',{}) for r in results]);cost=s['sdk_estimated_cost_usd']
        if abs(cost-u['recomputed_cost_usd'])>1e-5:audit.append({'run_id':rid,'issue':'cost_mismatch','sdk':cost,'recomputed':u['recomputed_cost_usd']})
        if u['cache_creation_input_tokens']!=u['cache_write_1h']+u['cache_write_5m']:audit.append({'run_id':rid,'issue':'unpriced_cache'})
        if u['output_tokens']!=record['result']['total_output_tokens']:audit.append({'run_id':rid,'issue':'controller_output_mismatch'})
        for e in s['rate_limits']:rates[json.dumps(e['message'].get('rate_limit_info',{}),sort_keys=True)]+=1
        wp=[i for i,c in enumerate(calls) if c.name.endswith('move_package_wp')];ed=[i for i,c in enumerate(calls) if c.name in ('Edit','Write')];verify=[i for i,c in enumerate(calls) if c.name.endswith(('move_package_verify','move_spec_check'))]
        row={**base,**u,'sdk_cost_usd':cost,'terminal_status':summary['terminal_status'],'success':int(bool(summary['operational_success'])),'wall_seconds':record['result']['controller_wall_ms']/1000,'api_seconds':mine['api_seconds'],'sdk_sessions':len(s['latest_result_by_session']),'sdk_queries':len(results),'model_turns':mine['model_turns'],'wp_calls':len(wp),'wp_before_first_edit':bool(wp and (not ed or wp[0]<ed[0])),'wp_after_first_verify':bool(wp and verify and wp[0]>verify[0]),'incomplete_queries':s['incomplete_query_count']}
        for k in ['tool_calls','verifier_calls','verifier_failures','compiler_failures','failure_kinds','whole_file_rewrites','targeted_edits','reverted_edit_pairs','repair_iterations','longest_gap_seconds']:row[k]=mine[k]
        ref=summary.get('refutation') or {};row.update(refutation_rounds=ref.get('runs',0),refutation_downgrades=ref.get('downgrades',0),refutation_inconclusive=ref.get('inconclusive',0))
        report=summary['final_report'];row['self_report_mentions_wp']=bool(__import__('re').search(r'\bWP\b|weakest.precondition',report,__import__('re').I));row['report_path']=str(d.relative_to(out) if d.is_relative_to(out) else Path('..')/'runs'/rid)
        rows.append(row)
        reports=[]
        for sid,last in s['latest_result_by_session'].items():
            rr=[r for r in results if r.get('session_id')==sid];au=usage_totals([r.get('usage',{}) for r in rr]);attempts.append({**base,'session_id':sid,**au,'sdk_cost_usd':last.get('total_cost_usd'),'api_seconds':(last.get('duration_api_ms') or 0)/1000,'queries':len(rr)})
            reports.extend({'session_id':sid,'query':i+1,'text':r.get('result','')} for i,r in enumerate(rr))
        # Arguments/results can contain private source: retain raw transcripts in place.
        details.append({'metrics':row,'tools':mine['tool_call_counts'],'self_reports':reports,'final_report':report,'refutation':ref,'tool_sequence':[{'sequence':c.sequence,'utc_ms':c.utc_ms,'name':c.name,'failed':c.failed} for c in calls]})
    done=[r for r in rows if 'sdk_cost_usd' in r];partial=[]
    for p in sorted((root/'interrupted-concurrency4').rglob('sdk-metrics.json')):
        s=json.loads(p.read_text());u=usage_totals([x.get('usage',{}) for x in s['api_messages_by_id'].values()]);partial.append({'path':str(p.relative_to(root)),**u,'result_count':s['result_count'],'sdk_cost_usd':s['sdk_estimated_cost_usd'],'incomplete_queries':s['incomplete_query_count']})
    numeric=FIELDS+['total_input_tokens','total_tokens','thinking_tokens','sdk_cost_usd','recomputed_cost_usd','wall_seconds','api_seconds','tool_calls','wp_calls','verifier_calls','verifier_failures','model_turns','sdk_sessions','sdk_queries']
    totals={k:sum(r[k] for r in done) for k in numeric}
    metrics=['sdk_cost_usd','output_tokens','total_input_tokens','wall_seconds','success']
    arms={arm:{'sessions':len(rr:=[r for r in done if r['arm']==arm]),'successes':sum(r['success'] for r in rr),'totals':{k:sum(r[k] for r in rr) for k in numeric},'mean_ci':{k:cluster(rr,k) for k in metrics},'wp_sessions':sum(r['wp_calls']>0 for r in rr),'wp_before_edit':sum(r['wp_before_first_edit'] for r in rr),'wp_after_verify':sum(r['wp_after_first_verify'] for r in rr),'verifier_failure_sessions':sum(r['verifier_failures']>0 for r in rr)} for arm in ARMS}
    blocks=collections.defaultdict(dict)
    for r in done:blocks[(r['task_id'],r['replicate'])][r['arm']]=r
    complete={k:v for k,v in blocks.items() if len(v)==3};contrasts={}
    for name,left,right in [('C1','hybrid_flexible','agent_only'),('C2','hybrid_guided','hybrid_flexible'),('C3','hybrid_guided','agent_only')]:
        diffs=[{'task_id':k[0],**{m:v[left][m]-v[right][m] for m in metrics}} for k,v in complete.items()]
        contrasts[name]={'definition':left+' minus '+right,'blocks':len(diffs),'metrics':{m:cluster(diffs,m) for m in metrics}}
    allci={k:cluster(done,k) for k in metrics}
    data={'totals':totals,'all_mean_ci':allci,'arms':arms,'contrasts':contrasts,'audit':audit,'rate_limit_info_counts':dict(rates),'interrupted':partial,'scheduled':len(rows),'recorded':len(done),'missing':len(rows)-len(done),'bootstrap':{'seed':20260907,'resamples':10000,'unit':'task','estimand':'equal-weight mean of observed within-task session means','interval':'percentile','scope':'exploratory; outcome-dependent truncation'}}
    dump(out/'analysis.json',data);dump(out/'session-details.json',details);csv_write(out/'per-session.csv',rows);csv_write(out/'per-sdk-session.csv',attempts);csv_write(out/'per-query.csv',queries);csv_write(out/'interrupted-startup.csv',partial)
    dump(out/'pricing.json',{'currency':'USD','rates_per_million_tokens':PRICE,'source':'https://platform.claude.com/docs/en/about-claude/pricing','retrieved_date':'2026-09-07','billing_meaning':'API-equivalent estimate; subscription invoice unavailable','thinking':'subset of output, not an additional charge'})
    print(json.dumps(data,indent=2))
    from analysis.opus_report_render import render
    render(root,out,data,rows,details)

if __name__=='__main__':main()
