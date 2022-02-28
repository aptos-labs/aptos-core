#!/bin/sh

# Script to be run in a Forge pod after the test runs as cleanup
# This separates the pod lifecycle from diem/diem

# CoreDNS in EKS needs to scale down to avoid cluster resource contention
# after the cluster has been scaled down by forge test cleanup
kubectl scale deployment/coredns --namespace kube-system --replicas=2
