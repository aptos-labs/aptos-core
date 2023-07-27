## Pangu

> Pangu (Chinese: 盤古, PAN-koo) is a primordial being and creation figure in Chinese mythology who separated heaven [testnet formation] and earth [infrastructure] and became geographic features such as mountains and rivers.

![Pangu](https://miro.medium.com/v2/resize:fit:1100/format:webp/1*tOtbmC03M3VBhl32yRo0mg.jpeg)

Pangu is a testnet creation and management CLI, which deploys on top of existing infrastructure.

## Dev Setup

Since this repo has a few separate stacks, setup can be split into different steps:

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
