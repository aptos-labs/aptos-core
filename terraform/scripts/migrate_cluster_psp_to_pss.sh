#!/usr/bin/env bash

function msg() {
  if [[ ${VERBOSE} == true ]]; then
    echo "$@" 2>&1
  fi
}

function disable_psp_ns() {
  local _ns=${1}
  msg "Disabling PodSecurityPolicy on namespace ${_ns}"
  kubectl delete -n "${_ns}" rolebinding disable-psp 2> /dev/null
  kubectl create -n "${_ns}" rolebinding disable-psp \
    --clusterrole privileged-psp --group "system:serviceaccounts:${_ns}"
}

function set_pss_label() {
  local _ns=${1}
  local _policy=${2}
  msg "Namespace ${_ns}: setting policy ${_policy}"
  kubectl label --overwrite ns "${_ns}" "${_policy}"
}

function set_pss_labels_ns() {
  local _ns=${1}
  set_pss_label "${_ns}" "pod-security.kubernetes.io/enforce=privileged"
  set_pss_label "${_ns}" "pod-security.kubernetes.io/enforce-version=${POLICY_VERSION}"
  set_pss_label "${_ns}" "pod-security.kubernetes.io/warn=baseline"
  set_pss_label "${_ns}" "pod-security.kubernetes.io/warn-version=${POLICY_VERSION}"
  set_pss_label "${_ns}" "pod-security.kubernetes.io/audit=baseline"
  set_pss_label "${_ns}" "pod-security.kubernetes.io/audit-version=${POLICY_VERSION}"
}

function list_ns() {
  kubectl get ns | grep Active | awk '{ print $1 }'
}

function migrate() {
  msg "Creating resource PodSecurityPolicy/privileged-psp"
  local scriptdir
  scriptdir=$(dirname "$(readlink -f "${0}")")
  kubectl apply -f "${scriptdir}"/privileged-psp.yaml

  msg "Creating role 'privileged-psp'"
  kubectl delete clusterrole privileged-psp 2> /dev/null
  kubectl create clusterrole privileged-psp \
    --verb use --resource podsecuritypolicies --resource-name privileged-psp

  local _ns
  for _ns in $(list_ns); do
    disable_psp_ns "${_ns}"
    # set_pss_labels_ns "${_ns}" "${POLICY_VERSION}"
  done
  set_pss_labels_ns default "${POLICY_VERSION}"
}

function clean() {
  msg "Cleaning up PSP resources"
  kubectl delete clusterrole privileged-psp 2> /dev/null

  local _ns
  for _ns in $(list_ns); do
    kubectl delete -n "${_ns}" rolebinding disable-psp 2> /dev/null
  done
}

POLICY_VERSION=v1.24
VERBOSE=false
cmd=""

optspec="h-:"
while getopts "$optspec" optchar; do
  case "${optchar}" in
    -)
      case "${OPTARG}" in
        debug)
          set +x
          ;;
        verbose)
          VERBOSE=true
          ;;
        policy-version=*)
          val=${OPTARG#*=}
          POLICY_VERSION=${val}
          ;;
        *)
          if [ "$OPTERR" = 1 ] && [ "${optspec:0:1}" != ":" ]; then
            echo "Unknown option --${OPTARG}" >&2
          fi
          ;;
      esac
      ;;
    *)
      echo "Unknown argument: '-${OPTARG}'" >&2
      exit 2
      ;;
  esac
done
shift $((OPTIND - 1))

case $# in
  0)
    cmd="usage"
    ;;
  1)
    cmd="${1}"
    ;;
  *)
    echo "Too many parameters on the command line" >&2
    exit 2
    ;;
esac

case "${cmd}" in
  usage)
    echo "Usage: $(basename "${0}") [--verbose] [--debug] [--policy-version=<value>] check | migrate | clean" >&2
    echo "Default PSS policy version: ${POLICY_VERSION}" >&2
    exit 1
    ;;
  check)
    echo "Hint: you can get the list of labels with kubectl get ns --show-labels"
    kubectl label --dry-run=server \
      --overwrite ns --all \
      pod-security.kubernetes.io/enforce=baseline
    ;;
  clean)
    clean
    ;;
  migrate)
    migrate
    ;;
  *)
    echo "Unknown command: ${cmd}"
    exit 2
    ;;
esac
