from argparse import ArgumentParser
from collections import defaultdict
import config
import re
import util


def get_cmd(config):

    build_args_map = {
        'release': '--release',
        'performance': '--profile performance',
        'debug': '',
    }

    build_args = build_args_map[config["build"]]

    if config['block_gen']['txn_gen']['_type'] == 'connected_groups':
        txn_gen_args = f"--connected-tx-grps {config['block_gen']['txn_gen']['num_groups']}"
        if config['block_gen']['txn_gen']['shuffle']:
            txn_gen_args += " --shuffle-connected-txns"
    elif config['block_gen']['txn_gen']['_type'] == 'p2p_sample':
        txn_gen_args = f"--hotspot-probability {config['block_gen']['txn_gen']['hotspot']}"
    else:
        txn_gen_args = ''

    if config['approach']['_type'] == 'unsharded':
        return f"""V3B=0 cargo run {build_args} -p aptos-executor-benchmark -- \
        --block-size {config['block_gen']['block_size']} \
        {txn_gen_args} \
        --execution-threads {config['approach']['concurrency']} \
        --split-ledger-db --use-sharded-state-merkle-db --skip-index-and-usage \
        run-executor \
        --blocks 100 --main-signer-accounts {config['block_gen']['txn_gen']['account_pool']} \
        --data-dir {config['db']['db']} --checkpoint-dir {config['db']['ck']} | grep TTTT
"""

    if config['approach']['_type'] == 'graph':
        return f"""cargo run {build_args} -p aptos-executor-benchmark -- \
        --block-size {config['block_gen']['block_size']} \
        {txn_gen_args} \
        --use-sharding-v3 \
        --num-executor-shards {config['approach']['shards']} \
        --execution-threads {config['approach']['shard_concurrency'] * config['approach']['shards']} \
        --split-ledger-db --use-sharded-state-merkle-db --skip-index-and-usage \
        run-executor \
        --blocks 100 --main-signer-accounts {config['block_gen']['txn_gen']['account_pool']} \
        --data-dir {config['db']['db']} --checkpoint-dir {config['db']['ck']} | grep TTTT
"""

    if config['approach']['_type'] == 'orderer':
        return f"""V3B=1 V3B__MAX_WINDOW_SIZE={config['approach']['orderer_config']['window_size']} cargo run {build_args} -p aptos-executor-benchmark -- \
        --block-size {config['block_gen']['block_size']} \
        {txn_gen_args} \
        --use-sharding-v3 \
        --num-executor-shards {config['approach']['shards']} \
        --execution-threads {config['approach']['shard_concurrency'] * config['approach']['shards']} \
        --split-ledger-db --use-sharded-state-merkle-db --skip-index-and-usage \
        run-executor \
        --blocks 100 --main-signer-accounts {config['block_gen']['txn_gen']['account_pool']} \
        --data-dir {config['db']['db']} --checkpoint-dir {config['db']['ck']} | grep TTTT
"""

    assert False


def main(config, spans):
    print(f'config={config}')
    cmd = get_cmd(config)
    proc = util.invoke(cmd)
    stdout = proc.stdout.decode('utf-8')
    lines = stdout.strip().split('\n')
    datapoints = {}
    for line in lines:
        match = re.match(r'\[TTTT\] block_id=(?P<BlockId>\w+), (?P<SpanName>\w+)=(?P<ScopeTime>(\d+))ms', line)
        if match:
            span_name = match.group('SpanName')
            block_id = match.group('BlockId')
            scope_ms = int(match.group('ScopeTime'))
            datapoints_for_span = datapoints.setdefault(span_name, {})
            datapoints_for_span[block_id] = max(datapoints_for_span.get(block_id, 0), scope_ms)
        else:
            print(f'line={line}')
            assert False
    for span in spans:
        scopes_by_block = datapoints[span]
        durations = list(scopes_by_block.values())
        durations.sort()
        num_blocks = len(durations)
        print(f'span={span}, AVG={sum(durations)/num_blocks}, P50={durations[num_blocks//2]}, P80={durations[-num_blocks//5]}, P95={durations[-num_blocks//20]}')
    print()
    print()


if __name__ == '__main__':
    parser = ArgumentParser()
    parser.add_argument('spans', nargs='+')
    args = parser.parse_args()

    for config in config.all_configs():
        main(config, args.spans)
