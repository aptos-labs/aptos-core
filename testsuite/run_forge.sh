#!/bin/bash

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

#
# This script runs Forge using the current configured kubectl context
# It is designed to be invoked within Github Actions, but can be run locally
# if the necessary environment variables are set.
#

# Default to off
WRAPPER_KILLSWITCH="${WRAPPER_KILLSWITCH:-true}"

# output files
FORGE_OUTPUT="${FORGE_OUTPUT:-$(mktemp)}"
FORGE_REPORT="${FORGE_REPORT:-$(mktemp)}"
FORGE_PRE_COMMENT="${FORGE_PRE_COMMENT:-$(mktemp)}"
FORGE_COMMENT="${FORGE_COMMENT:-$(mktemp)}"

if [[ $WRAPPER_KILLSWITCH = true ]]; then
    echo "Using current forge wrapper"
else
    echo "Running new forge wrapper"
    export FORGE_INSTALL_DEPENDENCIES=yeet
    exec python3 testsuite/forge.py test "$@"
fi

# available clusters to choose from
# forge-1 is used for continuous testing exclusively
FORGE_CLUSTERS=("aptos-forge-0" "aptos-forge-2")

# ensure the script is run from project root
pwd | grep -qE 'aptos-core$' || (echo "Please run from aptos-core root directory" && exit 1)

# for calculating regression in local mode
LOCAL_P99_LATENCY_MS_THRESHOLD=60000


# cluster auth
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
FORGE_NAMESPACE_REUSE=${FORGE_NAMESPACE_REUSE:-false}
FORGE_ENABLE_HAPROXY=${FORGE_ENABLE_HAPROXY:-false}
FORGE_TEST_SUITE=${FORGE_TEST_SUITE:-land_blocking}
FORGE_RUNNER_DURATION_SECS=3600
FORGE_RUNNER_TPS_THRESHOLD=${FORGE_RUNNER_TPS_THRESHOLD:-400}

[ "$FORGE_NAMESPACE_REUSE" = "true" ] && REUSE_ARGS="--reuse"
[ "$FORGE_NAMESPACE_KEEP" = "true" ] && KEEP_ARGS="--keep"
[ "$FORGE_ENABLE_HAPROXY" = "true" ] && ENABLE_HAPROXY_ARGS="--enable-haproxy"

print_output_files() {
    echo "FORGE_OUTPUT: ${FORGE_OUTPUT}"
    echo "FORGE_REPORT: ${FORGE_REPORT}"
    echo "FORGE_PRE_COMMENT: ${FORGE_PRE_COMMENT}"
    echo "FORGE_COMMENT: ${FORGE_COMMENT}"
}

# Set variables for o11y resource locations depending on the type of cluster that is running Forge
set_o11y_resources() {
    if echo $FORGE_CLUSTER_NAME | grep "forge"; then
        ES_DEFAULT_INDEX=$DEVINFRA_ES_DEFAULT_INDEX
        ES_BASE_URL=$DEVINFRA_ES_BASE_URL
        GRAFANA_BASE_URL=$DEVINFRA_GRAFANA_BASE_URL
    else
        ES_DEFAULT_INDEX=$INTERN_ES_DEFAULT_INDEX
        ES_BASE_URL=$INTERN_ES_BASE_URL
        GRAFANA_BASE_URL=$INTERN_GRAFANA_BASE_URL
    fi
}

# Set the k8s namespace in which to execute Forge tests
set_forge_namespace() {
    if [ -z "$FORGE_NAMESPACE" ]; then
        namespace="forge-${USER}-$(date '+%s')"
        echo "FORGE_NAMESPACE not set, using auto-generated namespace: ${namespace}"
    fi
    FORGE_NAMESPACE=${FORGE_NAMESPACE:-$namespace}
    # it must be under 64 chars alphanumeric
    FORGE_NAMESPACE="${FORGE_NAMESPACE//[^[:alnum:]]/-}"
    FORGE_NAMESPACE=${FORGE_NAMESPACE:0:64}
}

# Either: (1) confirms the given $IMAGE_TAG exists, or (2) searches the git history to set:
#  LATEST_IMAGE_TAG -- the latest built image
#  SECOND_LATEST_IMAGE_TAG -- the second latest built image
get_image_tags() {
    echo "Ensure image exists"
    if [ -n "$IMAGE_TAG" ]; then
        img=$(aws ecr describe-images --repository-name="aptos/validator" --image-ids=imageTag=$IMAGE_TAG 2>/dev/null)
        if [ "$?" -ne 0 ]; then
            echo "IMAGE_TAG does not exist in ECR: ${IMAGE_TAG}. Make sure your commit has been pushed to GitHub previously."
            echo "If you're trying to run the code from your PR, apply the label 'CICD:build-images' and wait for the builds to finish."
            echo
            echo "Attempting to find the latest commits before HEAD that have been built..."
            echo
        fi
    fi
    commit_threshold=20
    # get the LATEST_IMAGE_TAG
    for i in $(seq 0 $commit_threshold); do
        IMAGE_TAG_DEFAULT=$(git rev-parse HEAD~$i)
        echo "Trying tag: ${IMAGE_TAG_DEFAULT}"
        git --no-pager log --format=%B -n 1 $IMAGE_TAG_DEFAULT
        img=$(aws ecr describe-images --repository-name="aptos/validator" --image-ids=imageTag=$IMAGE_TAG_DEFAULT 2>/dev/null)
        if [ "$?" -eq 0 ]; then
            LATEST_IMAGE_TAG=$IMAGE_TAG_DEFAULT
            break
        fi
    done
    # get the SECOND_LATEST_IMAGE_TAG
    for i in $(seq 1 $commit_threshold); do
        # start at the latest built image tag, and search backwards from there
        IMAGE_TAG_DEFAULT=$(git rev-parse $LATEST_IMAGE_TAG~$i)
        echo "Trying tag: ${IMAGE_TAG_DEFAULT}"
        git --no-pager log --format=%B -n 1 $IMAGE_TAG_DEFAULT
        img=$(aws ecr describe-images --repository-name="aptos/validator" --image-ids=imageTag=$IMAGE_TAG_DEFAULT 2>/dev/null)
        if [ "$?" -eq 0 ]; then
            SECOND_LATEST_IMAGE_TAG=$IMAGE_TAG_DEFAULT
            break
        fi
    done
    # if IMAGE_TAG still not set after checking HEAD,
    if [ -z "$LATEST_IMAGE_TAG" ]; then
        echo "None of the last ${commit_threshold} commits have been built and pushed"
        exit 1
    fi
}

# Once o11y resource locations setup, build a link to the validator logs
get_validator_logs_link() {
    # build the logs link in a readable way...
    # filter by:
    #   * chain_name: name of the Forge cluster/chain
    #   * namespace: kubernetes namespace the Forge test was executed in
    #   * hostname: name of a kubernetes pod e.g. validator name
    if [ -n "$ENABLE_LOG_AUTO_REFRESH" ]; then
        ES_TIME_FILTER="refreshInterval:(pause:!f,value:10000),time:(from:now-15m,to:now)"
    else
        ES_TIME_FILTER="refreshInterval:(pause:!t,value:0),time:(from:'${ES_START_TIME}',to:'${ES_END_TIME}')"
    fi
    VAL0_HOSTNAME="aptos-node-0-validator-0"
    VALIDATOR_LOGS_LINK="${ES_BASE_URL}/_dashboards/app/discover#/?
        _g=(filters:!(),${ES_TIME_FILTER})
        &_a=(columns:!(_source),filters:!(
            ('\$state':(store:appState),meta:(alias:!n,disabled:!f,index:'${ES_DEFAULT_INDEX}',key:chain_name,negate:!f,params:(query:${FORGE_CHAIN_NAME}),type:phrase),query:(match_phrase:(chain_name:${FORGE_CHAIN_NAME}))),
            ('\$state':(store:appState),meta:(alias:!n,disabled:!f,index:'${ES_DEFAULT_INDEX}',key:namespace,negate:!f,params:(query:${FORGE_NAMESPACE}),type:phrase),query:(match_phrase:(namespace:${FORGE_NAMESPACE}))),
            ('\$state':(store:appState),meta:(alias:!n,disabled:!f,index:'${ES_DEFAULT_INDEX}',key:hostname,negate:!f,params:(query:${VAL0_HOSTNAME}),type:phrase),query:(match_phrase:(hostname:${VAL0_HOSTNAME})))
        ),index:'${ES_DEFAULT_INDEX}',interval:auto,query:(language:kuery,query:''),sort:!())"

    # trim all the whitespace in logs link
    VALIDATOR_LOGS_LINK=$(echo $VALIDATOR_LOGS_LINK | tr -d '[:space:]')
}

get_dashboard_link() {
    if echo $FORGE_CLUSTER_NAME | grep -qv "forge"; then
        FORGE_CHAIN_NAME="${FORGE_CHAIN_NAME}net"
    fi
    if [ -n "$ENABLE_DASHBOARD_AUTO_REFRESH" ]; then
        GRAFANA_TIME_FILTER="&refresh=10s&from=now-15m&to=now"
    else
        GRAFANA_TIME_FILTER="&from=${FORGE_START_TIME_MS}&to=${FORGE_END_TIME_MS}"
    fi
    FORGE_DASHBOARD_LINK="${GRAFANA_BASE_URL}&var-namespace=${FORGE_NAMESPACE}&var-chain_name=${FORGE_CHAIN_NAME}${GRAFANA_TIME_FILTER}"
}

print_output_files

if [ -z "$FORGE_CLUSTER_NAME" ]; then
    FORGE_CLUSTER_NAME=${FORGE_CLUSTERS[ $RANDOM % ${#FORGE_CLUSTERS[@]} ]}
fi

aws eks update-kubeconfig --name $FORGE_CLUSTER_NAME

# determine cluster name from kubectl context and set o11y resources
echo "Using cluster ${FORGE_CLUSTER_NAME}"
echo "Note: the current kubectl context has changed"
# remove the "aptos-" prefix and add "net" suffix to get the chain name
# as used by the deployment setup and as reported to o11y systems
FORGE_CHAIN_NAME=${FORGE_CLUSTER_NAME#"aptos-"}
HUMIO_START_TIME_MS=$(date +%s000)
HUMIO_END_TIME_MS=$(($HUMIO_START_TIME_MS + 1000 * 60 * 45)) # 45 min later

# set the namespace in FORGE_NAMESPACE
set_forge_namespace

set_humio_logs_link() {
    HUMIO_LOGS_LINK="https://cloud.us.humio.com/k8s/search?query=%24forgeLogs%28validator_instance%3D%2A%29%20%7C%20$FORGE_NAMESPACE%20&live=false&start=$HUMIO_START_TIME_MS&end=$HUMIO_END_TIME_MS&widgetType=list-view&columns=%5B%7B%22type%22%3A%22field%22%2C%22fieldName%22%3A%22%40timestamp%22%2C%22format%22%3A%22timestamp%22%2C%22width%22%3A180%7D%2C%7B%22type%22%3A%22field%22%2C%22fieldName%22%3A%22level%22%2C%22format%22%3A%22text%22%2C%22width%22%3A54%7D%2C%7B%22type%22%3A%22link%22%2C%22openInNewBrowserTab%22%3Atrue%2C%22style%22%3A%22button%22%2C%22hrefTemplate%22%3A%22https%3A%2F%2Fgithub.com%2Faptos-labs%2Faptos-core%2Fpull%2F%7B%7Bfields%5B%5C%22github_pr%5C%22%5D%7D%7D%22%2C%22textTemplate%22%3A%22%7B%7Bfields%5B%5C%22github_pr%5C%22%5D%7D%7D%22%2C%22header%22%3A%22Forge%20PR%22%2C%22width%22%3A79%7D%2C%7B%22type%22%3A%22field%22%2C%22fieldName%22%3A%22k8s.namespace%22%2C%22format%22%3A%22text%22%2C%22width%22%3A104%7D%2C%7B%22type%22%3A%22field%22%2C%22fieldName%22%3A%22k8s.pod_name%22%2C%22format%22%3A%22text%22%2C%22width%22%3A126%7D%2C%7B%22type%22%3A%22field%22%2C%22fieldName%22%3A%22k8s.container_name%22%2C%22format%22%3A%22text%22%2C%22width%22%3A85%7D%2C%7B%22type%22%3A%22field%22%2C%22fieldName%22%3A%22message%22%2C%22format%22%3A%22text%22%7D%5D&newestAtBottom=true&showOnlyFirstLine=false"
}


get_image_tags

echo "Calculated image tags:"
echo "  latest: $LATEST_IMAGE_TAG"
echo "  second latest: $SECOND_LATEST_IMAGE_TAG"

# Set the relative image tags for each components, dependent on the test suite
if [ "$FORGE_TEST_SUITE" = "compat" ]; then
    # if we're doing compat tests, start at the second latest, and upgrade to the latest
    FORGE_IMAGE_TAG=${FORGE_IMAGE_TAG:-$LATEST_IMAGE_TAG}
    IMAGE_TAG=${IMAGE_TAG:-$SECOND_LATEST_IMAGE_TAG}
    UPGRADE_IMAGE_TAG=${UPGRADE_IMAGE_TAG:-$LATEST_IMAGE_TAG}
else
    # otherwise, start at the latest always
    FORGE_IMAGE_TAG=${FORGE_IMAGE_TAG:-$LATEST_IMAGE_TAG}
    IMAGE_TAG=${IMAGE_TAG:-$LATEST_IMAGE_TAG}
    UPGRADE_IMAGE_TAG=${UPGRADE_IMAGE_TAG:-$LATEST_IMAGE_TAG}
fi

echo "Testing image tags:"
echo "  forge: $FORGE_IMAGE_TAG"
echo "  base: $IMAGE_TAG"
echo "  upgrade (if applicable): $UPGRADE_IMAGE_TAG"

# set the o11y resource locations in
# ES_DEFAULT_INDEX, ES_BASE_URL, GRAFANA_BASE_URL
set_o11y_resources
ENABLE_LOG_AUTO_REFRESH=true get_validator_logs_link
ENABLE_DASHBOARD_AUTO_REFRESH=true get_dashboard_link
# construct a pre-comment
cat <<EOF >$FORGE_PRE_COMMENT
### Forge is running with \`${IMAGE_TAG}\`
* [Grafana dashboard (auto-refresh)]($FORGE_DASHBOARD_LINK)
* [Humio Logs]($HUMIO_LOGS_LINK)
* [(Deprecated) OpenSearch Logs]($VALIDATOR_LOGS_LINK)
EOF
echo "=====START PRE_FORGE COMMENT====="
cat $FORGE_PRE_COMMENT
echo "=====END PRE_FORGE COMMENT====="

FORGE_START_TIME_MS="$(date '+%s')000"
ES_START_TIME=$(TZ=UTC date +"%Y-%m-%dT%H:%M:%S.000Z")

# # Run forge with test runner in k8s
if [ "$FORGE_RUNNER_MODE" = "local" ]; then

    # more file descriptors for heavy txn generation
    ulimit -n 1048576

    # port-forward prometheus
    kubectl port-forward -n default svc/aptos-node-mon-aptos-monitoring-prometheus 9090:9090 >/dev/null 2>&1 &
    prometheus_port_forward_pid=$!

    cargo run -p forge-cli -- --suite $FORGE_TEST_SUITE --mempool-backlog 5000 --avg-tps $FORGE_RUNNER_TPS_THRESHOLD \
        --max-latency-ms $LOCAL_P99_LATENCY_MS_THRESHOLD --duration-secs $FORGE_RUNNER_DURATION_SECS \
        test k8s-swarm \
        --image-tag $IMAGE_TAG \
        --upgrade-image-tag $UPGRADE_IMAGE_TAG \
        --namespace $FORGE_NAMESPACE \
        --port-forward $REUSE_ARGS $KEEP_ARGS $ENABLE_HAPROXY_ARGS | tee $FORGE_OUTPUT

    FORGE_EXIT_CODE=$?

    # try to kill orphaned port-forwards
    if [ -z "$KEEP_ARGS" ]; then
        ps -A | grep "port-forward -n $FORGE_NAMESPACE" | awk '{ print $1 }' | xargs -I{} kill -9 {}
        kill -9 $prometheus_port_forward_pid
    fi

elif [ "$FORGE_RUNNER_MODE" = "k8s" ]; then
    # try deleting pod corresponding to the same forge test (namespace, image_tag/git_ref)
    # this will pre-empt the existing forge test and ensures we do not have any dangling test runners
    FORGE_POD_NAME="${FORGE_NAMESPACE}-$(date '+%s')-${IMAGE_TAG}"
    FORGE_POD_NAME=${FORGE_POD_NAME:0:64}
    kubectl delete pod -n default -l "forge-namespace=${FORGE_NAMESPACE}" --force || true
    kubectl wait -n default --for=delete pod -l "forge-namespace=${FORGE_NAMESPACE}" || true

    specfile=$(mktemp)
    echo "Forge test-runner pod Spec : ${specfile}"

    [[ "$GITHUB_ACTIONS" == "true" ]] && FORGE_TRIGGERED_BY=github-actions || FORGE_TRIGGERED_BY=other

    sed -e "s/{FORGE_POD_NAME}/${FORGE_POD_NAME}/g" \
        -e "s/{FORGE_TEST_SUITE}/${FORGE_TEST_SUITE}/g" \
        -e "s/{FORGE_RUNNER_DURATION_SECS}/${FORGE_RUNNER_DURATION_SECS}/g" \
        -e "s/{FORGE_RUNNER_TPS_THRESHOLD}/${FORGE_RUNNER_TPS_THRESHOLD}/g" \
        -e "s/{FORGE_IMAGE_TAG}/${FORGE_IMAGE_TAG}/g" \
        -e "s/{IMAGE_TAG}/${IMAGE_TAG}/g" \
        -e "s/{UPGRADE_IMAGE_TAG}/${UPGRADE_IMAGE_TAG}/g" \
        -e "s/{AWS_ACCOUNT_NUM}/${AWS_ACCOUNT_NUM}/g" \
        -e "s/{AWS_REGION}/${AWS_REGION}/g" \
        -e "s/{FORGE_NAMESPACE}/${FORGE_NAMESPACE}/g" \
        -e "s/{REUSE_ARGS}/${REUSE_ARGS}/g" \
        -e "s/{KEEP_ARGS}/${KEEP_ARGS}/g" \
        -e "s/{ENABLE_HAPROXY_ARGS}/${ENABLE_HAPROXY_ARGS}/g" \
        -e "s/{FORGE_TRIGGERED_BY}/${FORGE_TRIGGERED_BY}/g" \
        testsuite/forge-test-runner-template.yaml >${specfile}

    kubectl apply -n default -f $specfile

    # wait for enough time for the pod to start and potentially new nodes to come online
    kubectl wait -n default --timeout=5m --for=condition=Ready "pod/${FORGE_POD_NAME}"

    # tail the logs and tee them for further parsing
    echo "=====START FORGE LOGS====="
    kubectl logs -n default -f $FORGE_POD_NAME | tee $FORGE_OUTPUT
    echo "=====END FORGE COMMENT====="

    # wait for the pod status to change potentially
    sleep 10
    while [[ $(kubectl get pods $FORGE_POD_NAME -o 'jsonpath={..status.conditions[?(@.type=="Ready")].status}') == "True" ]]; do echo "waiting for pod to complete: $FORGE_POD_NAME" && sleep 1; done

    # parse the pod status: https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle/#pod-phase
    forge_pod_status=$(kubectl get pod -n default $FORGE_POD_NAME -o jsonpath="{.status.phase}" 2>&1)
    echo "Forge pod status: ${forge_pod_status}"

    if [ "$forge_pod_status" = "Succeeded" ]; then # the current pod succeeded
        FORGE_EXIT_CODE=0
    elif echo $forge_pod_status | grep -E "(not found)|(NotFound)|(No such)"; then # the current test in this namespace was likely preempted and deleted
        FORGE_EXIT_CODE=10
    else # it did not succeed
        FORGE_EXIT_CODE=1
    fi
elif [ "$FORGE_RUNNER_MODE" = "dry-run" ]; then
    # assume you already have the local report and output files
    FORGE_EXIT_CODE=0
elif [ "$FORGE_RUNNER_MODE" = "pre-forge" ]; then
    # perform the pre-forge checks first and exit cleanly
    exit 0
else
    echo "Invalid FORGE_RUNNER_MODE: ${FORGE_RUNNER_MODE}"
    exit 1
fi

FORGE_END_TIME_MS="$(date '+%s')000"
ES_END_TIME=$(TZ=UTC date +"%Y-%m-%dT%H:%M:%S.000Z")

# parse the JSON report
# also handle test report failure cases
cat $FORGE_OUTPUT | awk '/====json-report-begin===/{f=1;next} /====json-report-end===/{f=0} f' >"${FORGE_REPORT}"
# If no report was generated, fill with default report
if [ ! -s "${FORGE_REPORT}" ]; then
    echo '{"text": "Forge test runner terminated"}' >"${FORGE_REPORT}"
fi
# If no report text was generated, fill with default text
FORGE_REPORT_TXT=$(cat $FORGE_REPORT | jq -r .text)
if [ -z "$FORGE_REPORT_TXT" ]; then
    FORGE_REPORT_TXT="Forge report text empty. See test runner output."
fi

# print the Forge report
cat $FORGE_REPORT

[ -z "$GITHUB_RUN_ID" ] && GITHUB_RUN_ID=0
AVG_TPS=$(cat $FORGE_REPORT | grep -oE '[0-9]+ TPS' | awk '{print $1}')
P99_LATENCY=$(cat $FORGE_REPORT | grep -oE '[0-9]+ ms p99 latency' | awk '{print $1}')
if [ -n "$AVG_TPS" ]; then
    echo "AVG_TPS: ${AVG_TPS}"
    echo "forge_job_avg_tps {FORGE_CLUSTER_NAME=\"$FORGE_CLUSTER_NAME\",FORGE_NAMESPACE=\"$FORGE_NAMESPACE\",GITHUB_RUN_ID=\"$GITHUB_RUN_ID\"} $AVG_TPS" | curl -u "$PUSH_GATEWAY_USER:$PUSH_GATEWAY_PASSWORD" --data-binary @- ${PUSH_GATEWAY}/metrics/job/forge
fi
if [ -n "$P99_LATENCY" ]; then
    echo "P99_LATENCY: ${P99_LATENCY}"
    echo "forge_job_p99_latency {FORGE_CLUSTER_NAME=\"$FORGE_CLUSTER_NAME\",FORGE_NAMESPACE=\"$FORGE_NAMESPACE\",GITHUB_RUN_ID=\"$GITHUB_RUN_ID\"} $P99_LATENCY" | curl -u "$PUSH_GATEWAY_USER:$PUSH_GATEWAY_PASSWORD" --data-binary @- ${PUSH_GATEWAY}/metrics/job/forge
fi

# Get the final o11y links that are not auto-refresh
get_dashboard_link
get_validator_logs_link
HUMIO_END_TIME_MS=$FORGE_END_TIME_MS
set_humio_logs_link

# construct forge comment output
if [ "$FORGE_EXIT_CODE" = "0" ]; then
    FORGE_COMMENT_HEADER="### :white_check_mark: Forge test success on \`${IMAGE_TAG}\`"
elif [ "$FORGE_EXIT_CODE" = "2" ]; then
    FORGE_COMMENT_HEADER="### :x: Forge test perf regression on \`${IMAGE_TAG}\`"
elif [ "$FORGE_EXIT_CODE" = "10" ]; then
    FORGE_COMMENT_HEADER="### :thought_balloon: Forge test preempted on \`${IMAGE_TAG}\`"
else
    FORGE_COMMENT_HEADER="### :x: Forge test failure on \`${IMAGE_TAG}\`"
fi
cat <<EOF >$FORGE_COMMENT
$FORGE_COMMENT_HEADER
\`\`\`
$FORGE_REPORT_TXT
\`\`\`
* [Grafana dashboard]($FORGE_DASHBOARD_LINK)
* [Humio Logs]($HUMIO_LOGS_LINK)
* [(Deprecated) OpenSearch Logs]($VALIDATOR_LOGS_LINK)
EOF

echo "=====START FORGE COMMENT====="
cat $FORGE_COMMENT
echo "=====END FORGE COMMENT====="

echo "Forge exit with: $FORGE_EXIT_CODE"

print_output_files

# report metrics to pushgateway
echo "forge_job_status {FORGE_EXIT_CODE=\"$FORGE_EXIT_CODE\",FORGE_CLUSTER_NAME=\"$FORGE_CLUSTER_NAME\",FORGE_NAMESPACE=\"$FORGE_NAMESPACE\"} $GITHUB_RUN_ID" | curl -u "$PUSH_GATEWAY_USER:$PUSH_GATEWAY_PASSWORD" --data-binary @- ${PUSH_GATEWAY}/metrics/job/forge
