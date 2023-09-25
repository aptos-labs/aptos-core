from collections import defaultdict
import re
import util

builds = [
    'performance',
    # 'debug',
]

dbs = [
    # {
    #     'db': '/tmp/db2',
    #     'ck': '/tmp/ck2',
    # },
    {
        'db': '/tmp/jmtexp/some-db',
        'ck': '/tmp/jmtexp/some-ck',
    },
]

block_gens = [
    # {
    #     'block_size': 100,
    #     'txn_gen': {
    #         '_type': "p2p_sample",
    #         'account_pool': 100,
    #         "hotspot": 0.5,
    #     },
    # },
    {
        'block_size': 10000,
        'txn_gen': {
            '_type': "p2p_sample",
            'account_pool': 10000,
            "hotspot": 0.5,
        },
    },
    {
        'block_size': 10000,
        'txn_gen': {
            '_type': "p2p_sample",
            'account_pool': 10000,
            "hotspot": 0.8,
        },
    },
    {
        'block_size': 10000,
        'txn_gen': {
            '_type': "p2p_sample",
            'account_pool': 10000,
            "hotspot": 0.99,
        },
    },
    # {
    #     'block_size': 10000,
    #     'txn_gen': {
    #         '_type': "p2p_sample",
    #         'account_pool': 100000,
    #         "hotspot": 0.5,
    #     },
    # },
    # {
    #     'block_size': 10000,
    #     'txn_gen': {
    #         '_type': "p2p_sample",
    #         'account_pool': 100000,
    #         "hotspot": 0.8,
    #     },
    # },
    # {
    #     'block_size': 10000,
    #     'txn_gen': {
    #         '_type': "p2p_sample",
    #         'account_pool': 100000,
    #         "hotspot": 0.99,
    #     },
    # },
    {
        'block_size': 10000,
        'txn_gen': {
            '_type': "p2p_sample",
            'account_pool': 1000000,
            "hotspot": 0.5,
        },
    },
    {
        'block_size': 10000,
        'txn_gen': {
            '_type': "p2p_sample",
            'account_pool': 1000000,
            "hotspot": 0.8,
        },
    },
    {
        'block_size': 10000,
        'txn_gen': {
            '_type': "p2p_sample",
            'account_pool': 1000000,
            "hotspot": 0.99,
        },
    },
    {
        'block_size': 10000,
        'txn_gen': {
            '_type': "connected_groups",
            'account_pool': 100000,
            "num_groups": 500,
            "shuffle": True,
        },
    },
    {
        'block_size': 10000,
        'txn_gen': {
            '_type': "connected_groups",
            'account_pool': 100000,
            "num_groups": 1,
            "shuffle": True,
        },
    },
    # {
    #     'block_size': 10000,
    #     'txn_gen': {
    #         '_type': "connected_groups",
    #         'account_pool': 100000,
    #         "num_groups": 10,
    #         "shuffle": True,
    #     },
    # },
    # {
    #     'block_size': 10000,
    #     'txn_gen': {
    #         '_type': "connected_groups",
    #         'account_pool': 100000,
    #         "num_groups": 100,
    #         "shuffle": False,
    #     },
    # },
    # {
    #     'block_size': 10000,
    #     'txn_gen': {
    #         '_type': "connected_groups",
    #         'account_pool': 100000,
    #         "num_groups": 1000,
    #         "shuffle": False,
    #     },
    # },
]

approaches = [
    # {
    #     "_type": "unsharded",
    #     "concurrency": 60,
    # },
    # {
    #     "_type": "unsharded",
    #     "concurrency": 56,
    # },
    # {
    #     "_type": "unsharded",
    #     "concurrency": 52,
    # },
    # {
    #     "_type": "unsharded",
    #     "concurrency": 48,
    # },
    # {
    #     "_type": "unsharded",
    #     "concurrency": 44,
    # },
    # {
    #     "_type": "unsharded",
    #     "concurrency": 40,
    # },
    # {
    #     "_type": "unsharded",
    #     "concurrency": 36,
    # },
    # {
    #     "_type": "unsharded",
    #     "concurrency": 32,
    # },
    # {
    #     "_type": "unsharded",
    #     "concurrency": 28,
    # },
    # {
    #     "_type": "unsharded",
    #     "concurrency": 24,
    # },
    # {
    #     "_type": "unsharded",
    #     "concurrency": 20,
    # },
    # {
    #     "_type": "unsharded",
    #     "concurrency": 16,
    # },
    # {
    #     "_type": "unsharded",
    #     "concurrency": 12,
    # },
    # {
    #     "_type": "unsharded",
    #     "concurrency": 8,
    # },
    # {
    #     "_type": "sharded_v1",
    #     "xshard_avoid": 0.9,
    #     "max_rounds": 2,
    #     "global_shard": True,
    #     "executor_shards": 48,
    #     "shard_concurrency": 1,
    # },
    # {
    #     "_type": "sharded_v2_cc",
    #     "load_imba": 2.0,
    #     "partitioner_concurrency": 4,
    #     "dashmap_shards": 16,
    #     "max_rounds": 2,
    #     "global_shard": True,
    #     "executor_shards": 48,
    #     "shard_concurrency": 1,
    # },
    # {
    #     "_type": "sharded_v2_cc",
    #     "load_imba": 4.0,
    #     "partitioner_concurrency": 32,
    #     "dashmap_shards": 256,
    #     "max_rounds": 2,
    #     "global_shard": True,
    #     "executor_shards": 48,
    #     "shard_concurrency": 1,
    # },
    # {
    #     "_type": "ptx",
    #     "concurrency": 54,
    # },
    # {
    #     "_type": "graph",
    #     "shards": 2,
    #     "shard_concurrency": 24,
    # },
    {
        "_type": "graph",
        "shards": 3,
        "shard_concurrency": 16,
    },
    {
        "_type": "graph",
        "shards": 4,
        "shard_concurrency": 12,
    },
    # {
    #     "_type": "graph",
    #     "shards": 6,
    #     "shard_concurrency": 8,
    # },
    # {
    #     "_type": "graph",
    #     "shards": 8,
    #     "shard_concurrency": 6,
    # },
    # {
    #     "_type": "graph",
    #     "shards": 12,
    #     "shard_concurrency": 4,
    # },
    # {
    #     "_type": "graph",
    #     "shards": 16,
    #     "shard_concurrency": 3,
    # },
]


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
        return f"""cargo run {build_args} -p aptos-executor-benchmark -- \
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

    assert False


def main(config):
    print(f'config={config}')
    cmd = get_cmd(config)
    proc = util.invoke(cmd)
    stdout = proc.stdout.decode('utf-8')
    lines = stdout.strip().split('\n')
    scopes_by_block = defaultdict(int)
    for line in lines:
        match = re.match(r'\[TTTT\] block_id=(?P<BlockId>\w+), blockstm_threads=(?P<ScopeTime>(\d+))ms', line)
        if match:
            block_id = match.group('BlockId')
            scope_ms = int(match.group('ScopeTime'))
            scopes_by_block[block_id] = max(scopes_by_block[block_id], scope_ms)
        else:
            print(f'line={line}')
            assert False
    durations = list(scopes_by_block.values())
    durations.sort()
    num_blocks = len(durations)
    print(f'AVG={sum(durations)/num_blocks}, P50={durations[num_blocks//2]}, P80={durations[-num_blocks//5]}, P95={durations[-num_blocks//20]}')
    print()
    print()


if __name__ == '__main__':
    for build in builds:
        for db in dbs:
            for approach in approaches:
                for block_gen in block_gens:
                    quick_config = {
                        'approach': approach,
                        'block_gen': block_gen,
                        'build': build,
                        'db': db,
                    }
                    main(quick_config)
