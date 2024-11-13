This script orchestrates the replay and verification of blockchain data using Kubernetes pods. It defines a WorkerPod class to manage individual pods, handling their status, logs, and environment variables. The ReplayScheduler class schedules tasks for these pods, ensuring they run sequentially while managing retries, logging, and error handling. It supports scheduling from specific blockchain versions, skipping defined ranges, and collecting logs from failed or mismatched transactions. The script uses Kubernetes API for pod management and includes configurable hyperparameters for sharding, retries, concurrency, and delays. The main function initializes the scheduler and starts the scheduling process from scratch.

## Prerequiste 
Install minikube

## Local test
minikube start --mount --mount-string="/mnt/testnet_archive:/mnt/testnet_archive"  --memory=81920 --cpus=17
minikb apply -f ./testnet-archive.yaml

poetry shell
poetry install # install kubenetes 
poetry run