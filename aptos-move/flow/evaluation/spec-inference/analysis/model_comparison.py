"""Collect the frozen UC model comparison without changing evaluation outcomes."""
import collections, csv, dataclasses, json, random, statistics
from pathlib import Path
from analysis.round_summary import collect_run
from harness.mine import analyze_run, _tool_calls

ROOT=Path(__file__).resolve().parent.parent
OUT=ROOT/'evaluation-artifacts/corpus3.2-model-compare-uc'

def collect():
    rows=[];details=[];audit=[]
    for model in ['glm','opus']:
        rd=OUT.with_name(OUT.name+'-'+model)
        for manifest in sorted((rd/'schedule/runs').glob('*.json')):
            spec=json.loads(manifest.read_text()); run=rd/'runs'/spec['run_id']
            j=json.loads((run/'judge.json').read_text());sdk=json.loads((run/'sdk-metrics.json').read_text())
            summary=collect_run(run);mine=dataclasses.asdict(analyze_run(run));calls=list(_tool_calls(run/'claude-events.jsonl'))
            usages=[x['result'].get('usage',{}) for x in sdk['results']]
            tokens={k:sum(u.get(k,0) or 0 for u in usages) for k in ['input_tokens','cache_creation_input_tokens','cache_read_input_tokens','output_tokens']}
            thinking=sum((u.get('output_tokens_details') or {}).get('thinking_tokens',0) or 0 for u in usages)
            row={'model':model,'arm':spec['arm'],'replicate':spec['replicate'],'run_id':spec['run_id'],'terminal_status':j['terminal_status'],'operational_success':j['operational_success'],**tokens,'reported_thinking_tokens':thinking,'total_input_tokens':sum(tokens[k] for k in list(tokens)[:3]),'sdk_estimated_cost_usd':sdk['sdk_estimated_cost_usd'],'wall_seconds':j['controller_wall_ms']/1000,'api_seconds':mine['api_seconds'],'sdk_queries':sdk['result_count'],'sdk_sessions':len(sdk['latest_result_by_session']),'usage_complete':sdk['totals_complete'],'model_turns':mine['model_turns'],'tool_calls':mine['tool_calls'],'edits':sum(c.name in ('Edit','Write') for c in calls),'wp_calls':sum(c.name.endswith('move_package_wp') for c in calls),'verifier_calls':mine['verifier_calls'],'verifier_failures':mine['verifier_failures'],'compiler_failures':mine['compiler_failures'],'repair_iterations':mine['repair_iterations']}
            local=collections.Counter();wp_events=[]
            for line in (run/'flow-events.jsonl').read_text().splitlines():
                e=json.loads(line)
                if e.get('event')=='tool_end':
                    local[e['tool_name']]+=e.get('duration_us',0)/1e6
                    if e['tool_name']=='move_package_wp':wp_events.append({k:e.get(k) for k in ['sequence','duration_us','outcome','response_bytes']})
            row['local_mcp_seconds']=sum(local.values());row['wp_seconds']=local['move_package_wp']
            row['sdk_cost_formula_usd']=(tokens['input_tokens']*5+tokens['cache_read_input_tokens']*.5+tokens['output_tokens']*25+sum((u.get('cache_creation') or {}).get('ephemeral_1h_input_tokens',0)*10+(u.get('cache_creation') or {}).get('ephemeral_5m_input_tokens',0)*6.25 for u in usages))/1e6
            if abs(row['sdk_cost_formula_usd']-row['sdk_estimated_cost_usd'])>1e-6:audit.append({'run_id':row['run_id'],'issue':'SDK cost formula mismatch'})
            scorepath=run/'mutation-score.json'
            if scorepath.exists():
                score=json.loads(scorepath.read_text());row.update(essential_mutants=score['essential_mutants'],killed=score['killed'],mutation_adequacy=score['mutation_adequacy'])
                row['strict_success']=bool(j['operational_success'] and score['killed']==score['essential_mutants'])
            else: row.update(essential_mutants=None,killed=None,mutation_adequacy=None,strict_success=False if not j['operational_success'] else None)
            row['restricted_output_to_success']=row['output_tokens'] if row['operational_success'] else 150000
            row['restricted_wall_to_success']=row['wall_seconds'] if row['operational_success'] else 3600
            if row['output_tokens']!=j['total_output_tokens']:audit.append({'run_id':row['run_id'],'issue':'output reconciliation'})
            if not row['usage_complete']:audit.append({'run_id':row['run_id'],'issue':'incomplete SDK usage'})
            final_by_session=sdk['latest_result_by_session'].values()
            models=[(key,value.get('costBasis')) for last in final_by_session for key,value in last.get('modelUsage',{}).items()]
            details.append({'metrics':row,'self_reports':[x['result'].get('result','') for x in sdk['results']],'final_report':summary['final_report'],'tools':mine['tool_call_counts'],'failure_kinds':mine['failure_kinds'],'refutation':summary['refutation'],'mutation':summary['mutation'],'wp_events':wp_events,'local_tool_seconds':dict(local),'native_models_and_cost_basis':models,'tool_sequence':[{'sequence':c.sequence,'name':c.name,'failed':c.failed} for c in calls]})
            rows.append(row)
    groups=[]
    for model in ['glm','opus']:
        for arm in ['agent_only','hybrid_guided']:
            rs=sorted([r for r in rows if r['model']==model and r['arm']==arm],key=lambda r:r['replicate'])
            g={'model':model,'arm':arm,'n':len(rs),'successes':sum(r['operational_success'] for r in rs),'strict_successes':sum(bool(r['strict_success']) for r in rs)}
            for key in ['output_tokens','total_input_tokens','input_tokens','cache_creation_input_tokens','cache_read_input_tokens','reported_thinking_tokens','sdk_estimated_cost_usd','wall_seconds','api_seconds','local_mcp_seconds','wp_seconds','model_turns','tool_calls','edits','wp_calls','verifier_calls','verifier_failures','compiler_failures','restricted_output_to_success','restricted_wall_to_success']:
                vs=[r[key] for r in rs];g[key]={'total':sum(vs),'mean':statistics.mean(vs),'median':statistics.median(vs)}
                if key in ('output_tokens','sdk_estimated_cost_usd','wall_seconds'):
                    boot_rng=random.Random(20260907)
                    boot=sorted(statistics.mean(boot_rng.choices(vs,k=4)) for _ in range(10000))
                    g[key].update(low95=boot[249],high95=boot[9749])
            groups.append(g)
    rng=random.Random(20260907);draws=collections.defaultdict(list)
    for _ in range(10000):
        ratios={};differences={}
        for model in ['glm','opus']:
            indices=rng.choices([1,2,3,4],k=4)
            a=[next(r for r in rows if r['model']==model and r['arm']=='agent_only' and r['replicate']==i)['output_tokens'] for i in indices]
            h=[next(r for r in rows if r['model']==model and r['arm']=='hybrid_guided' and r['replicate']==i)['output_tokens'] for i in indices]
            ratios[model]=statistics.mean(h)/statistics.mean(a);differences[model]=statistics.mean(h)-statistics.mean(a)
            draws[model+'_guided_agent_ratio'].append(ratios[model]);draws[model+'_guided_minus_agent_output'].append(differences[model])
        draws['ratio_of_ratios_opus_over_glm'].append(ratios['opus']/ratios['glm'])
        draws['output_interaction_opus_minus_glm'].append(differences['opus']-differences['glm'])
    cis={k:{'low95':sorted(v)[249],'high95':sorted(v)[9749]} for k,v in draws.items()}
    data={'rows':rows,'groups':groups,'bootstrap_intervals':cis,'audit':audit}
    (OUT/'analysis.json').write_text(json.dumps(data,indent=2)+'\n');(OUT/'session-details.json').write_text(json.dumps(details,indent=2)+'\n')
    with (OUT/'per-session.csv').open('w') as f:
        w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
    print(json.dumps({'groups':groups,'bootstrap_intervals':cis,'audit':audit},indent=2))

if __name__=='__main__': collect()
