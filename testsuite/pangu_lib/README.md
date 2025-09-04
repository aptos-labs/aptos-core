## Pangu

> Pangu (Chinese: 盤古, PAN-koo) is a primordial being and creation figure in Chinese mythology who separated heaven [testnet formation] and earth [infrastructure] and became geographic features such as mountains and rivers.

![Pangu](https://miro.medium.com/v2/resize:fit:1100/format:webp/1*tOtbmC03M3VBhl32yRo0mg.jpeg)

Pangu is a testnet creation and management CLI, which deploys on top of existing infrastructure.

## What is Pangu CLI?

Ever had to wait for the Velor devnet/testnet releases to test a new feature? Or, create a PR to launch testnets through Forge? Well, these will be a thing of the past with Pangu.

Pangu is a modular, customizable, and next-gen Velor testnet creation-management CLI tool written in Python. Pangu allows you to create, and manage testnets on demand, and blazingly fast 🚀🚀🚀 

Pangu is inherently faster than its predecessors (Forge testnet creation) because:

- Pangu does not use Helm
- Pangu introduces new optimizations using concurrency/parallelism

Also, Pangu’s source code is significantly easier to read because it is written in strictly-typed Python 3.0, and in a modular manner. 

## Vision

Pangu is meant to be used by researchers/devs that want to create a testnet quickly. But, Pangu also aims to replace how testnets are created and deployed in Forge. 

The Forge integrations are outside of the scope of the initial iteration built by @Olsen Budanur , but (🤞) Forge will eventually call the Pangu CLI to do testnet creation, and management. 

## Main Functionalities

Here is a brief overview of all the commands offered by the Pangu CLI. **For more information about the options + arguments, use pangu [testnet/node] [command] -h.** 

### **Testnet Functions**

pangu testnet create [OPTIONS] : Creates a testnet with the configurations given in the options in the connected cluster

pangu testnet delete [TESTNET_NAME] : Deletes testnet in the connected cluster

pangu testnet get: Displays all active testnets in the connected cluster

pangu testnet get [TESTNET_NAME]: Displays the nodes of a singular testnet in the connected cluster

pangu testnet healthcheck [TESTNET_NAME]: Healthcheck a singular testnet in the connected cluster (WIP)

pangu testnet restart [TESTNET_NAME]: Restart all nodes in a singular testnet in the connected cluster

pangu testnet update [TESTNET_NAME] [OPTIONS]: Update all nodes in a singular testnet in the connected cluster using the options 

pangu testnet transaction-emitter [TESTNET_NAME] [OPTIONS]: Create a transaction emitter for a testnet by name.

### **Node Functions**

pangu node stop [TESTNET_NAME] [NODE_NAME]: Stops a nodes in a singular testnet in the connected cluster

pangu node start [TESTNET_NAME] [NODE_NAME]: Starts a nodes in a singular testnet in the connected cluster

pangu node restart [TESTNET_NAME] [NODE_NAME]: Restarts a nodes in a singular testnet in the connected cluster

pangu node profile [TESTNET_NAME] [NODE_NAME]: Shows you node profiling tools created by @Yunus Ozer 

pangu node wipe [TESTNET_NAME] [NODE_NAME]: Wipes a nodes in a singular testnet in the connected cluster (WIP)

pangu node add-pfn [TESTNET_NAME] [NODE_NAME] [OPTIONS]: Adds a pfn in a singular testnet in the connected cluster using the options (WIP)

## More Info About the “Create” Command

pangu testnet create [OPTIONS] : Creates a testnet with the configurations given in the options in the connected cluster

CREATE OPTIONS:

1. **`-pangu-node-configs-path`**:
    - The Pangu node configs (yaml)
    - Default: The default node config in velor-core/testsuite/pangu_lib/template_testnet_files
    - Example: **`-pangu-node-configs-path /path/to/node/configs.yaml`**
2. **`-layout-path`**:
    - The path to the layout file (yaml).
    - Default: The default layout in velor-core/testsuite/pangu_lib/template_testnet_files
    - Example: **`-layout-path /path/to/layout.yaml`**
3. **`-framework-path`**:
    - The compiled move framework (head.mrb, or framework.mrb) file. Defaults to the default framework in the pangu_lib.
    - Default: **`util.TEMPLATE_DIRECTORY/framework.mrb`**
    - Example: **`-framework-path /path/to/framework.mrb`**
4. **`-num-of-validators`**:
    - The number of generic validators you would like to have in the testnet. This option will be overwritten if you are passing custom Pangu node configs
    - Default: **`10`**
    - Example: **`-num-of-validators 20`**
5. **`-workspace`**:
    - The path to the folder you would like the genesis files to be generated (default is a temp folder).
    - Example: **`-workspace /path/to/workspace`**
- **`-dry-run`**:
    - Pass **`true`** if you would like to run genesis without deploying on Kubernetes (K8S). All Kubernetes YAML files will be dumped to the workspace. If you don’t provide a workspace, all the YAML files will be dumped to a tmp folder.
    - Default: **`False`**
    - Example: **`-dry-run true`**
1. **`-velor-cli-path`**:
    - The path to the Velor CLI if it is not in your $PATH variable.
    - Default: **`velor`**
    - Example: **`-velor-cli-path /path/to/velor`**
2. **`-name`**:
    - Name for the testnet. The default is a randomly generated name. The name will automatically have “pangu-” appended to it.
    - Example: **`-name MyTestnet`**


## Pangu Node Config (Customizability)

[Pangu config template](https://github.com/velor-chain/velor-core/blob/main/testsuite/pangu_lib/template_testnet_files/pangu_node_config.yaml)

```yaml
blueprints:
  nodebp: # Must to be all lowercase, and distinct
    validator_config_path: "" # Should provide an absolute path. Can leave empty for the default
    validator_image: "" # Can leave empty for the default
    validator_storage_class_name: "" # Can leave empty for the default
    vfn_config_path: "" # Should provide an absolute path. Use empty str if create_vfns: false. # Can leave empty for the default
    vfn_image: ""  # Can leave empty for the default
    vfn_storage_class_name: "" # Can leave empty for the default
    nodes_persistent_volume_claim_size: "" # Can leave empty for the default
    create_vfns: true # CANNOT BE MODIFIED AFTER DEPLOYMENT
    stake_amount: 100000000000000 # CANNOT BE MODIFIED AFTER DEPLOYMENT
    count: -1 # CANNOT BE MODIFIED AFTER DEPLOYMENT... This is count of validators. In the template, the count doesn't matter as it gets overriden by either the default (10), user's --num-of-validators, or user's custom pangue node config.
  # nodebpexample1: 
  #   validator_config_path: ""
  #   validator_image: "" 
  #   validator_storage_class_name: "" # Can leave empty for the default
  #   vfn_config_path: "" 
  #   vfn_image: ""
  #   nodes_persistent_volume_claim_size: "" # Can leave empty for the default 
  #   create_vfns: false # 
  #   stake_amount: 100000000000000 
  #   count: -1
  # nodebpexample2: 
  #   validator_config_path: ""
  #   validator_image: "" 
  #   validator_storage_class_name: "" # Can leave empty for the default
  #   vfn_config_path: "" 
  #   nodes_persistent_volume_claim_size: "" # Can leave empty for the default 
  #   vfn_image: "" 
  #   vfn_storage_class_name: "" # Can leave empty for the default
  #   create_vfns: false # 
  #   stake_amount: 100000000000000 
  #   count: -1
```

Pangu allows you to use a default template to create n number of nodes without much customization. However, if you want to create a testnet with varying node configurations and pod images, this is also possible through a custom pangu config.

To create a testnet with a custom topology, create a new pangu config file and pass it with the option "--pangu-node-configs-path" 

- [**See the default config here**](https://github.com/velor-chain/velor-core/blob/main/testsuite/pangu_lib/template_testnet_files/pangu_node_config.yaml)
    - The config yaml should start with “blueprints:”
    - A blueprint describes the validator config, the validator image, the vfn config, the vfn image, stake_amount for the validator, and the number of validator/vfn pairs you would like to create with this specific blueprint.
    - The name of the blueprint will dictate the names of the pods (validators, vfns) created using it.
        - A validator created using the bp “nodebp” will be named nodebp-node-{i}-validator (i being the index of the validator. Likewise, a vfn created using the bp “nodebp” will be named nodebp-node-{i}-vfn.
    - You can (and for most cases, should) have multiple blueprints.
- The pangu configs are not only used for creating testnets, but also could be used to update one using the pangu testnet update command. You can change the image, and the node configs of a testnet that is already started by modifying your pangu node configs and using the testnet update command.

## How to Use Pangu

**1-** Have velor-core installed locally, and navigate to the testsuite directory. 

**2-** The entrypoint for all python operations is `[poetry](https://python-poetry.org/)`:

- Install poetry: [https://python-poetry.org/docs/#installation](https://python-poetry.org/docs/#installation)
- Install poetry deps: `poetry install`

**3-** To set up the pangu alias

```bash
alias pangu="poetry run python pangu.py"
```

**4-** Have a K8s environment set up. For testing purposes, I suggest you use KinD. [Here is a script that can be used to set up KinD.](https://github.com/velor-chain/internal-ops/blob/main/docker/kind/start-kind.sh)

**5-** Use “pangu -h”, “pangu node -h”, and “pangu testnet -h” commands to get more info about the Pangu commands

## Codebase

Pangu lives in velor-core/testsuite. Tips for navigating the codebase:

- [**velor-core/testsuite/pangu.py**](https://github.com/velor-chain/velor-core/blob/main/testsuite/pangu.py)
    - This is the entry point to the Pangu CLI. Use poetry run python pangu.py to run.
- [**velor-core/testsuite/test_framework**](https://github.com/velor-chain/velor-core/tree/main/testsuite/test_framework)
    - Includes the system abstractions for testing.
    - The Kubernetes abstraction might need to be updated to add new Kubernetes features.
- [**velor-core/testsuite/pangu_lib/node_commands**](https://github.com/velor-chain/velor-core/tree/main/testsuite/pangu_lib/node_commands)
    - Includes the commands for the pangu node {COMMAND} commands
    - Each command has its own .py file, which are then aggregated in the commands.py file to be exported to pangu.py
- [a**ptos-core/testsuite/pangu_lib/testnet_commands**](https://github.com/velor-chain/velor-core/tree/main/testsuite/pangu_lib/testnet_commands)
    - Includes the commands for the pangu testnet {COMMAND} commands
    - Each command has its own .py file, which are then aggregated in the commands.py file to be exported to pangu.py
- [**velor-core/testsuite/pangu_lib/tests**](https://github.com/velor-chain/velor-core/tree/main/testsuite/pangu_lib/tests)
    - Includes the unit tests
- [**velor-core/testsuite/pangu-sdk**](https://github.com/velor-chain/velor-core/tree/main/testsuite/pangu-sdk)
    - The Pangu Rust SDK is a light Rust wrapper around the Pangu CLI. It allows rust code to be able to run Pangu commands by passing structs, without having to generate the Pangu Config Yaml files. It is not feature complete, but should be a good starting point for the Pangu-Forge integrations.

## Metrics

@Olsen Budanur tested Pangu’s performance by creating testsnets of varying sizes in a standard GKE cluster. The table below shows how long it took Pangu to run genesis + apply all the k8s resources. 

Unlike Forge, Pangu was ran from a different cluster than where the testnet is deployed. Thus, it was disadvantaged in that regard during testing. 

|          Run |         4 Vals | 7 Vals + 5 VFNs | 100 Vals + 100 VFNs | 100 Vals + 0 VFNs |
| --- | --- | --- | --- | --- |
|             1 |             5 s |              9 s |                111 s |              65 s |
|             2 |             5 s |              12 s |                116 s |              65 s |
|             3 |             6 s |              10 s |                112 s |              65 s |
|             4 |             6 s |              10 s |                 111 s |              66 s |
|             5 |             5 s |              10 s |                 113 s |              63 s |
|             6 |             5 s |              10 s |                    x |                x |
|             7 |             5 s |              10 s |                    x |                x |
|             8 |             6 s |              13 s |                    x |                x |
|             9 |             6 s |              12 s |                    x |                x |
|            10 |             5 s |              12 s |                    x |                x |
| ———————— | ———————— | ————————— | ———————————— | ————————— |
|          AVG |           5.4 s |               10.8 s |               112.6 s |           64.8 s |
|    Forge AVG |         ~121 s |             ~148 s |                   x |              x |
|          Diff | Pangu ~22x Faster | Pangu ~14x Faster |                   x |              x |
|      Savings  |      *see below |      *see below |                   x |              x |

## Important Notes

- Pangu **DOES NOT** provision infrastructure. Being connected to a K8s cluster is a pre-req to using Pangu. Works with GKE node auto provisioning. 
- If you are getting cryptic errors, comment out the line below “UNCOMMENT FOR MORE VERBOSE ERROR MESSAGES” on pangu.py for more error information. All exceptions are routed to this code block.
    - You can also set all stream_output’s in create_testnet.py to be True to get even more logs.
- You should not rely too much on the default move framework mrb, and compile a new version often. An update to the move framework can, and has, break Pangu.


### Python

The entrypoint for all python operations is [`poetry`](https://python-poetry.org/):
* Install poetry: https://python-poetry.org/docs/#installation
* Install poetry deps: `poetry install`
* Activate virtualenv: `poetry run`
* Tools: `poetry run poe`

### Kubernetes

You will have a few kubernetes clusters to manage. At least one in GCP, one on your dev machine via KinD, etc.
* Install kubectl: https://kubernetes.io/docs/tasks/tools/install-kubectl-macos/
* Install KinD: https://kind.sigs.k8s.io/docs/user/quick-start/#installation
