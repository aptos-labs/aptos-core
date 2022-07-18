#!/bin/bash

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

#
# This script runs Forge using the current configured kubectl context
# It is designed to be invoked within Github Actions, but can be run locally
# if the necessary environment variables are set.
#

# ensure the script is run from project root
pwd | grep -qE 'aptos-core$' || (echo "Please run from aptos-core root directory" && exit 1)

# for calculating regression
TPS_THRESHOLD=4000
P99_LATENCY_MS_THRESHOLD=5000

FORGE_OUTPUT=${FORGE_OUTPUT:-forge_output.txt}
FORGE_REPORT=${FORGE_REPORT:-forge_report.json}
AWS_ACCOUNT_NUM=${AWS_ACCOUNT_NUM:-$(aws sts get-caller-identity | jq -r .Account)}
AWS_REGION=${AWS_REGION:-us-west-2}

# o11y resources
INTERN_ES_DEFAULT_INDEX="90037930-aafc-11ec-acce-2d961187411f"
INTERN_ES_BASE_URL="https://es.intern.aptosdev.com"
INTERN_GRAFANA_BASE_URL="https://o11y.aptosdev.com/grafana/d/overview/overview?orgId=1&refresh=10s&var-Datasource=Remote%20Prometheus%20Intern"
DEVINFRA_ES_DEFAULT_INDEX="d0bc5e20-badc-11ec-9a50-89b84ac337af"
DEVINFRA_ES_BASE_URL="https://es.devinfra.aptosdev.com"
DEVINFRA_GRAFANA_BASE_URL="https://o11y.aptosdev.com/grafana/d/overview/overview?orgId=1&refresh=10s&var-Datasource=Remote%20Prometheus%20Devinfra"

# forge test runner customization
FORGE_RUNNER_MODE=${FORGE_RUNNER_MODE:-k8s}
FORGE_NAMESPACE_KEEP=${FORGE_NAMESPACE_KEEP:-false}
FORGE_ENABLE_HAPROXY=${FORGE_ENABLE_HAPROXY:-false}

# if this script is not triggered in GHA, use a default value
[ -z "$GITHUB_RUN_ID" ] && GITHUB_RUN_ID=0

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


[ "$FORGE_NAMESPACE_KEEP" = "true" ] && KEEP_ARGS="--keep"
[ "$FORGE_ENABLE_HAPROXY" = "true" ] && ENABLE_HAPROXY_ARGS="--enable-haproxy"

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

# determine cluster name from kubectl context and set o11y resources
FORGE_CLUSTER_NAME=$(kubectl config current-context | grep -oE 'aptos.*')
if echo $FORGE_CLUSTER_NAME | grep "forge"; then
    ES_DEFAULT_INDEX=$DEVINFRA_ES_DEFAULT_INDEX
    ES_BASE_URL=$DEVINFRA_ES_BASE_URL
    GRAFANA_BASE_URL=$DEVINFRA_GRAFANA_BASE_URL
else
    ES_DEFAULT_INDEX=$INTERN_ES_DEFAULT_INDEX
    ES_BASE_URL=$INTERN_ES_BASE_URL
    GRAFANA_BASE_URL=$INTERN_GRAFANA_BASE_URL
fi

echo "Using cluster ${FORGE_CLUSTER_NAME} from current kubectl context"

FORGE_START_TIME_MS="$(date '+%s')000"
ES_START_TIME=$(TZ=UTC date +"%Y-%m-%dT%H:%M:%S.000Z")

# # Run forge with test runner in k8s
if [ "$FORGE_RUNNER_MODE" = "local" ]; then

    # more file descriptors for heavy txn generation
    ulimit -n 1048576

    cargo run -p forge-cli -- test k8s-swarm \
        --image-tag $IMAGE_TAG \
        --namespace $FORGE_NAMESPACE \
        --port-forward $KEEP_ARGS $ENABLE_HAPROXY_ARGS | tee $FORGE_OUTPUT

    FORGE_EXIT_CODE=$?

    # try to kill orphaned port-forwards
    if [ -z "$KEEP_ARGS" ]; then
        ps -A | grep  "kubectl port-forward -n $FORGE_NAMESPACE" | awk '{ print $1 }' | xargs -I{} kill -9 {}
    fi

elif [ "$FORGE_RUNNER_MODE" = "k8s" ]; then
    # try deleting existing forge pod of same name
    # since forge test runner will run in a pod that matches the namespace
    # this will pre-empt the existing forge test in the same namespace and ensures
    # we do not have any dangling test runners
    FORGE_POD_NAME=$FORGE_NAMESPACE
    kubectl delete pod $FORGE_POD_NAME || true
    kubectl wait --for=delete "pod/${FORGE_POD_NAME}" || true

    specfile=$(mktemp)
    echo "Forge test-runner pod Spec : ${specfile}"

    sed -e "s/{FORGE_POD_NAME}/${FORGE_POD_NAME}/g" \
        -e "s/{IMAGE_TAG}/${IMAGE_TAG}/g" \
        -e "s/{AWS_ACCOUNT_NUM}/${AWS_ACCOUNT_NUM}/g" \
        -e "s/{AWS_REGION}/${AWS_REGION}/g" \
        -e "s/{FORGE_NAMESPACE}/${FORGE_NAMESPACE}/g" \
        -e "s/{KEEP_ARGS}/${KEEP_ARGS}/g" \
        -e "s/{ENABLE_HAPROXY_ARGS}/${ENABLE_HAPROXY_ARGS}/g" \
        testsuite/forge-test-runner-template.yaml > ${specfile}
    
    kubectl apply -f $specfile

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
elif [ "$FORGE_RUNNER_MODE" = "dry-run" ]; then
    # assume you already have the local report and output files
    FORGE_EXIT_CODE=0
else
    echo "Invalid FORGE_RUNNER_MODE: ${FORGE_RUNNER_MODE}"
    exit 1
fi

FORGE_END_TIME_MS="$(date '+%s')000"
ES_END_TIME=$(TZ=UTC date +"%Y-%m-%dT%H:%M:%S.000Z")

# parse the JSON report
cat $FORGE_OUTPUT | awk '/====json-report-begin===/{f=1;next} /====json-report-end===/{f=0} f' >"${FORGE_REPORT}"
# If no report was generated, fill with default report
if [ ! -s "${FORGE_REPORT}" ]; then
    echo '{"text": "Forge test runner terminated"}' >"${FORGE_REPORT}"
fi

# print the Forge report
cat $FORGE_REPORT

# detect regressions
# TODO(rustielin): do not block on perf regressions for now until Forge network performance stabilizes
AVG_TPS=$(cat $FORGE_REPORT | grep -oE '[0-9]+ TPS' | awk '{print $1}')
P99_LATENCY=$(cat $FORGE_REPORT | grep -oE '[0-9]+ ms p99 latency' | awk '{print $1}')
if [ -n "$AVG_TPS" ]; then
    echo "AVG_TPS: ${AVG_TPS}"
    echo "forge_job_avg_tps {FORGE_CLUSTER_NAME=\"$FORGE_CLUSTER_NAME\",FORGE_NAMESPACE=\"$FORGE_NAMESPACE\",GITHUB_RUN_ID=\"$GITHUB_RUN_ID\"} $AVG_TPS" | curl -u "$PUSH_GATEWAY_USER:$PUSH_GATEWAY_PASSWORD" --data-binary @- ${PUSH_GATEWAY}/metrics/job/forge
    if [[ "$AVG_TPS" -lt "$AVG_TPS_MS_THRESHOLD" ]]; then
        echo "(\!) AVG_TPS: ${avg_tps} < ${TPS_THRESHOLD} tps"
    fi
fi
if [ -n "$P99_LATENCY" ]; then
    echo "P99_LATENCY: ${P99_LATENCY}"
    echo "forge_job_p99_latency {FORGE_CLUSTER_NAME=\"$FORGE_CLUSTER_NAME\",FORGE_NAMESPACE=\"$FORGE_NAMESPACE\",GITHUB_RUN_ID=\"$GITHUB_RUN_ID\"} $P99_LATENCY" | curl -u "$PUSH_GATEWAY_USER:$PUSH_GATEWAY_PASSWORD" --data-binary @- ${PUSH_GATEWAY}/metrics/job/forge
    if [[ "$P99_LATENCY" -gt "$P99_LATENCY_MS_THRESHOLD" ]]; then
        echo "(\!) P99_LATENCY: ${P99_LATENCY} > ${P99_LATENCY_MS_THRESHOLD} ms"
    fi
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

# remove the "aptos-" prefix and add "net" suffix to get the chain name
# as used by the deployment setup and as reported to o11y systems
FORGE_CHAIN_NAME=${FORGE_CLUSTER_NAME#"aptos-"}net
FORGE_DASHBOARD_LINK="${GRAFANA_BASE_URL}&var-namespace=${FORGE_NAMESPACE}&var-chain_name=${FORGE_CHAIN_NAME}&from=${FORGE_START_TIME_MS}&to=${FORGE_END_TIME_MS}"

# build the logs link in a readable way...
# filter by:
#   * chain_name: name of the Forge cluster "chain"
#   * namespace: kubernetes namespace the Forge test was executed in
#   * hostname: name of a kubernetes pod e.g. validator name
VAL0_HOSTNAME="aptos-node-0-validator-0"
VALIDATOR_LOGS_LINK="${ES_BASE_URL}/_dashboards/app/discover#/?
    _g=(filters:!(),refreshInterval:(pause:!t,value:0),time:(from:'${ES_START_TIME}',to:'${ES_END_TIME}'))
    &_a=(columns:!(_source),filters:!(
        ('\$state':(store:appState),meta:(alias:!n,disabled:!f,index:'${ES_DEFAULT_INDEX}',key:chain_name,negate:!f,params:(query:${FORGE_CHAIN_NAME}),type:phrase),query:(match_phrase:(chain_name:${FORGE_CHAIN_NAME}))),
        ('\$state':(store:appState),meta:(alias:!n,disabled:!f,index:'${ES_DEFAULT_INDEX}',key:namespace,negate:!f,params:(query:${FORGE_NAMESPACE}),type:phrase),query:(match_phrase:(namespace:${FORGE_NAMESPACE}))),
        ('\$state':(store:appState),meta:(alias:!n,disabled:!f,index:'${ES_DEFAULT_INDEX}',key:hostname,negate:!f,params:(query:${VAL0_HOSTNAME}),type:phrase),query:(match_phrase:(hostname:${VAL0_HOSTNAME})))
    ),index:'${ES_DEFAULT_INDEX}',interval:auto,query:(language:kuery,query:''),sort:!())"

# trim all the whitespace in logs link
VALIDATOR_LOGS_LINK=$(echo $VALIDATOR_LOGS_LINK | tr -d '[:space:]')

echo "=====START FORGE COMMENT====="
echo "$FORGE_COMMENT_HEADER"
echo "Dashboard: ${FORGE_DASHBOARD_LINK}"
echo "Logs: ${VALIDATOR_LOGS_LINK}"
echo '```'
echo "$FORGE_REPORT_TXT"
echo '```'
echo "=====END FORGE COMMENT====="

echo "Forge exit with: $FORGE_EXIT_CODE"

# report metrics to pushgateway
echo "forge_job_status {FORGE_EXIT_CODE=\"$FORGE_EXIT_CODE\",FORGE_CLUSTER_NAME=\"$FORGE_CLUSTER_NAME\",FORGE_NAMESPACE=\"$FORGE_NAMESPACE\"} $GITHUB_RUN_ID" | curl -u "$PUSH_GATEWAY_USER:$PUSH_GATEWAY_PASSWORD" --data-binary @- ${PUSH_GATEWAY}/metrics/job/forge
