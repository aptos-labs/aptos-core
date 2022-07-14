#!/bin/bash

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

#
# This script runs Forge using the current configured kubectl context
# It is designed to be invoked within Github Actions, but can be run locally
# if the necessary environment variables are set.
#

# for calculating regression
TPS_THRESHOLD=1500
P99_LATENCY_MS_THRESHOLD=5000

FORGE_OUTPUT=${FORGE_OUTPUT:-forge_output.txt}
FORGE_REPORT=${FORGE_REPORT:-forge_report.json}
AWS_ACCOUNT_NUM=${AWS_ACCOUNT_NUM:-$(aws sts get-caller-identity | jq -r .Account)}
AWS_REGION=${AWS_REGION:-us-west-2}

# forge test runner customization
FORGE_RUNNER_MODE=${FORGE_RUNNER_MODE:-k8s}
FORGE_NAMESPACE_KEEP=${FORGE_NAMESPACE_KEEP:-false}
FORGE_ENABLE_HAPROXY=${FORGE_ENABLE_HAPROXY:-true}

if [ -z "$FORGE_NAMESPACE" ]; then
    namespace="forge-${USER}-$(date '+%s')"
    echo "FORGE_NAMESPACE not set, using auto-generated namespace: ${namespace}"
fi
FORGE_NAMESPACE=${FORGE_NAMESPACE:-$namespace}
# clean up namespace name
# replace non alphanumeric chars with dash
FORGE_NAMESPACE="${FORGE_NAMESPACE//[^[:alnum:]]/-}"
# use the first 64 chars only for namespace, as it is the maximum for k8s resources
FORGE_NAMESPACE=${FORGE_NAMESPACE:0:64}


if [ "$FORGE_NAMESPACE_KEEP" = "true" ]; then
    KEEP_ARGS="--keep"
fi

if [ "$FORGE_ENABLE_HAPROXY" = "true" ]; then
    ENABLE_HAPROXY_ARGS="--enable-haproxy"
fi


if [ -z "$IMAGE_TAG" ]; then
    echo "IMAGE_TAG not set"
    exit 1
fi

echo "Ensure image exists"
img=$(aws ecr describe-images --repository-name="aptos/validator" --image-ids=imageTag=$IMAGE_TAG)
if [ $? != 0 ]; then
    echo "IMAGE_TAG does not exist: ${IMAGE_TAG}"
    exit 1
fi

FORGE_CLUSTER_NAME=$(kubectl config current-context | grep -oE 'aptos.*')

echo "Using cluster ${FORGE_CLUSTER_NAME} from current kubectl context"

FORGE_START_TIME_MS="$(date '+%s')000"

# # Run forge with test runner in k8s
if [ "$FORGE_RUNNER_MODE" = "local" ]; then

    # more file descriptors for heavy txn generation
    ulimit -n unlimited

    cargo run -p forge-cli -- test k8s-swarm \
        --image-tag $IMAGE_TAG \
        --namespace $FORGE_NAMESPACE \
        --port-forward $KEEP_ARGS $ENABLE_HAPROXY_ARGS | tee $FORGE_OUTPUT

    FORGE_EXIT_CODE=$?

    # try to kill orphaned port-forwards
    if [ -z "$KEEP_ARGS" ]; then
        ps -A | grep  "kubectl port-forward -n $FORGE_NAMESPACE" | awk '{ print $1 }' | xargs -I{} kill -9 {}
    fi

else
    # try deleting existing forge pod of same name
    # since forge test runner will run in a pod that matches the namespace
    # this will pre-empt the existing forge test in the same namespace and ensures
    # we do not have any dangling test runners
    FORGE_POD_NAME=$FORGE_NAMESPACE
    kubectl delete pod $FORGE_POD_NAME || true
    kubectl wait --for=delete "pod/${FORGE_POD_NAME}" || true

    # run a pod that runs the forge test runner
    # ensure that the pod uses for "forge" serviceAccount, and antiaffinity for other manually run forge pods
    kubectl run $FORGE_POD_NAME \
        --overrides='{"spec": { "serviceAccount": "forge", "affinity": { "podAntiAffinity": { "requiredDuringSchedulingIgnoredDuringExecution": [{ "labelSelector": { "matchExpressions": [{ "key": "run", "operator": "Exists" }]}, "topologyKey": "kubernetes.io/hostname" }] }}}}' \
        --restart=Never \
        --image="${AWS_ACCOUNT_NUM}.dkr.ecr.${AWS_REGION}.amazonaws.com/aptos/forge:$IMAGE_TAG" \
        --command -- bash -c "forge test k8s-swarm --image-tag $IMAGE_TAG --namespace $FORGE_NAMESPACE $KEEP_ARGS $ENABLE_HAPROXY_ARGS"

    # wait for enough time for the pod to start and potentially new nodes to come online
    kubectl wait --timeout=5m --for=condition=Ready "pod/${FORGE_POD_NAME}"

    # tail the logs and tee them for further parsing
    kubectl logs -f $FORGE_POD_NAME | tee $FORGE_OUTPUT

    # parse the pod status: https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle/#pod-phase
    forge_pod_status=$(kubectl get pod $FORGE_POD_NAME -o jsonpath="{.status.phase}")
    echo "Forge pod status: ${forge_pod_status}"
    if [ "$forge_pod_status" = "Succeeded" ]; then
        FORGE_EXIT_CODE=0
    else
        FORGE_EXIT_CODE=1
    fi
fi

FORGE_END_TIME_MS="$(date '+%s')000"

# parse the JSON report
cat $FORGE_OUTPUT | awk '/====json-report-begin===/{f=1;next} /====json-report-end===/{f=0} f' >"${FORGE_REPORT}"
# If no report was generated, fill with default report
if [ ! -s "${FORGE_REPORT}" ]; then
    echo '{"text": "Forge test runner terminated"}' >"${FORGE_REPORT}"
fi

# detect regressions
# TODO(rustielin): do not block on perf regressions for now until Forge network performance stabilizes
avg_tps=$(cat $FORGE_REPORT | grep -oE '\d+ TPS' | awk '{print $1}')
p99_latency=$(cat $FORGE_REPORT | grep -oE '\d+ ms p99 latency' | awk '{print $1}')
if [ -n "$avg_tps" ] && [ "$avg_tps" -lt "$TPS_THRESHOLD" ]; then
    echo "(\!) AVG_TPS: ${avg_tps} < ${TPS_THRESHOLD} tps"
    # FORGE_EXIT_CODE=1
fi
if [ -n "$p99_latency" ] && [ "$p99_latency" -gt "$P99_LATENCY_MS_THRESHOLD" ]; then
    echo "(\!) P99_LATENCY: ${p99_latency} > 5000 ms"
    # FORGE_EXIT_CODE=1
fi

# If no report text was generated, fill with default text
FORGE_REPORT_TXT=$(cat $FORGE_REPORT | jq -r .text)
if [ -z "$FORGE_REPORT_TXT" ]; then
    FORGE_REPORT_TXT="Forge report text empty. See test runner output."
    FORGE_EXIT_CODE=1
fi

if [ "$FORGE_EXIT_CODE" = "0" ]; then
    FORGE_COMMENT_HEADER='### :white_check_mark: Forge test success'
else
    FORGE_COMMENT_HEADER='### :x: Forge test failure'
fi

# remove the "aptos-" prefix to get the chain name as reported to Prometheus
FORGE_CHAIN_NAME=${FORGE_CLUSTER_NAME#"aptos-"}
FORGE_DASHBOARD_LINK="${GRAFANA_BASE_URL}&var-namespace=${FORGE_NAMESPACE}&var-chain_name=${FORGE_CHAIN_NAME}&from=${FORGE_START_TIME_MS}&to=${FORGE_END_TIME_MS}"
if [ -z "$GRAFANA_BASE_URL" ]; then
    echo "GRAFANA_BASE_URL not set. Use above query on supported Grafana dashboards"
fi

echo "=====START FORGE COMMENT====="
echo "$FORGE_COMMENT_HEADER"
echo "$FORGE_DASHBOARD_LINK"
echo '```'
echo "$FORGE_REPORT_TXT"
echo '```'
echo "=====END FORGE COMMENT====="

echo "Forge exit with: $FORGE_EXIT_CODE"
