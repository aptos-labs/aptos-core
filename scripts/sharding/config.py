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
    {
        'block_size': 10000,
        'txn_gen': {
            '_type': "p2p_sample",
            'account_pool': 100000,
            "hotspot": 0.5,
        },
    },
    {
        'block_size': 10000,
        'txn_gen': {
            '_type': "p2p_sample",
            'account_pool': 100000,
            "hotspot": 0.8,
        },
    },
    {
        'block_size': 10000,
        'txn_gen': {
            '_type': "p2p_sample",
            'account_pool': 100000,
            "hotspot": 0.99,
        },
    },
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
    # {
    #     "_type": "graph",
    #     "shards": 3,
    #     "shard_concurrency": 16,
    # },
    # {
    #     "_type": "graph",
    #     "shards": 4,
    #     "shard_concurrency": 12,
    # },
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
    {
        "_type": "orderer",
        "orderer_config": {
            "window_size": 1000,
        },
        "shards": 2,
        "shard_concurrency": 24,
    },
    {
        "_type": "orderer",
        "orderer_config": {
            "window_size": 500,
        },
        "shards": 2,
        "shard_concurrency": 24,
    },
    {
        "_type": "orderer",
        "orderer_config": {
            "window_size": 2000,
        },
        "shards": 2,
        "shard_concurrency": 24,
    },
]


def all_configs():
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
                    yield quick_config
