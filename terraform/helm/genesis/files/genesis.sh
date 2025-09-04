#!/bin/bash

#
# Runs an automated genesis ceremony for validators spun up by the velor-node helm chart
#
# Expect the following environment variables to be set before execution:
# NUM_VALIDATORS
# ERA
# WORKSPACE: default /tmp
# USERNAME_PREFIX: default velor-node
# VALIDATOR_INTERNAL_HOST_SUFFIX: default validator-lb
# FULLNODE_INTERNAL_HOST_SUFFIX: default fullnode-lb
#

set -x

WORKSPACE=${WORKSPACE:-/tmp}
USERNAME_PREFIX=${USERNAME_PREFIX:-velor-node}
VALIDATOR_INTERNAL_HOST_SUFFIX=${VALIDATOR_INTERNAL_HOST_SUFFIX:-validator-lb}
FULLNODE_INTERNAL_HOST_SUFFIX=${FULLNODE_INTERNAL_HOST_SUFFIX:-fullnode-lb}
MOVE_FRAMEWORK_DIR=${MOVE_FRAMEWORK_DIR:-"/velor-framework/move"}
STAKE_AMOUNT=${STAKE_AMOUNT:-1}
NUM_VALIDATORS_WITH_LARGER_STAKE=${NUM_VALIDATORS_WITH_LARGER_STAKE:0}
LARGER_STAKE_AMOUNT=${LARGER_STAKE_AMOUNT:-1}
# TODO: Fix the usage of this below when not set
RANDOM_SEED=${RANDOM_SEED:-$RANDOM}
export VELOR_DISABLE_TELEMETRY=true

ENABLE_MULTICLUSTER_DOMAIN_SUFFIX=${ENABLE_MULTICLUSTER_DOMAIN_SUFFIX:-false}
MULTICLUSTER_DOMAIN_SUFFIXES_DEFAULT="forge-multiregion-1,forge-multiregion-2,forge-multiregion-3"
MULTICLUSTER_DOMAIN_SUFFIXES_STRING=${MULTICLUSTER_DOMAIN_SUFFIXES_STRING:-${MULTICLUSTER_DOMAIN_SUFFIXES_DEFAULT}}
echo $MULTICLUSTER_DOMAIN_SUFFIXES_STRING
# convert comma separated string to array
IFS=',' read -r -a MULTICLUSTER_DOMAIN_SUFFIXES <<< "${MULTICLUSTER_DOMAIN_SUFFIXES_STRING}"

if ! [[ $(declare -p MULTICLUSTER_DOMAIN_SUFFIXES) =~ "declare -a" ]]; then
  echo "MULTICLUSTER_DOMAIN_SUFFIXES must be an array"
  exit 1
fi

if [[ "${ENABLE_MULTICLUSTER_DOMAIN_SUFFIX}" == "true" ]]; then
  if [ -z ${NAMESPACE} ]; then
    echo "NAMESPACE must be set"
    exit 1
  fi
fi

if [ -z ${ERA} ] || [ -z ${NUM_VALIDATORS} ]; then
  echo "ERA (${ERA:-null}) and NUM_VALIDATORS (${NUM_VALIDATORS:-null}) must be set"
  exit 1
fi

if [ "${FULLNODE_ENABLE_ONCHAIN_DISCOVERY}" = "true" ] && [ -z ${DOMAIN} ] \
  || [ "${VALIDATOR_ENABLE_ONCHAIN_DISCOVERY}" = "true" ] && [ -z ${DOMAIN} ]; then
  echo "If FULLNODE_ENABLE_ONCHAIN_DISCOVERY or VALIDATOR_ENABLE_ONCHAIN_DISCOVERY is set, DOMAIN must be set"
  exit 1
fi

if [ -z ${CLUSTER_NAME} ]; then
  echo "CLUSTER_NAME must be set"
  exit 1
fi

echo "NUM_VALIDATORS=${NUM_VALIDATORS}"
echo "ERA=${ERA}"
echo "WORKSPACE=${WORKSPACE}"
echo "USERNAME_PREFIX=${USERNAME_PREFIX}"
echo "VALIDATOR_INTERNAL_HOST_SUFFIX=${VALIDATOR_INTERNAL_HOST_SUFFIX}"
echo "FULLNODE_INTERNAL_HOST_SUFFIX=${FULLNODE_INTERNAL_HOST_SUFFIX}"
echo "STAKE_AMOUNT=${STAKE_AMOUNT}"
echo "NUM_VALIDATORS_WITH_LARGER_STAKE=${NUM_VALIDATORS_WITH_LARGER_STAKE}"
echo "LARGER_STAKE_AMOUNT=${LARGER_STAKE_AMOUNT}"
echo "RANDOM_SEED=${RANDOM_SEED}"

RANDOM_SEED_IN_DECIMAL=$(printf "%d" 0x${RANDOM_SEED})

# generate all validator configurations
for i in $(seq 0 $(($NUM_VALIDATORS - 1))); do
  username="${USERNAME_PREFIX}-${i}"
  user_dir="${WORKSPACE}/${username}"

  mkdir $user_dir

  if [[ "${FULLNODE_ENABLE_ONCHAIN_DISCOVERY}" = "true" ]]; then
    fullnode_host="fullnode${i}.${DOMAIN}:6182"
  elif [[ "${ENABLE_MULTICLUSTER_DOMAIN_SUFFIX}" = "true" ]]; then
    index=$(($i % ${#MULTICLUSTER_DOMAIN_SUFFIXES[@]}))
    cluster=${MULTICLUSTER_DOMAIN_SUFFIXES[${index}]}
    fullnode_host="${username}-${FULLNODE_INTERNAL_HOST_SUFFIX}.${NAMESPACE}.svc.${cluster}:6182"
  else
    fullnode_host="${username}-${FULLNODE_INTERNAL_HOST_SUFFIX}:6182"
  fi

  if [[ "${VALIDATOR_ENABLE_ONCHAIN_DISCOVERY}" = "true" ]]; then
    validator_host="val${i}.${DOMAIN}:6180"
  elif [[ "${ENABLE_MULTICLUSTER_DOMAIN_SUFFIX}" = "true" ]]; then
    index=$(($i % ${#MULTICLUSTER_DOMAIN_SUFFIXES[@]}))
    cluster=${MULTICLUSTER_DOMAIN_SUFFIXES[${index}]}
    validator_host="${username}-${VALIDATOR_INTERNAL_HOST_SUFFIX}.${NAMESPACE}.svc.${cluster}:6180"
  else
    validator_host="${username}-${VALIDATOR_INTERNAL_HOST_SUFFIX}:6180"
  fi

  if [ $i -lt $NUM_VALIDATORS_WITH_LARGER_STAKE ]; then
    CUR_STAKE_AMOUNT=$LARGER_STAKE_AMOUNT
  else
    CUR_STAKE_AMOUNT=$STAKE_AMOUNT
  fi

  echo "CUR_STAKE_AMOUNT=${CUR_STAKE_AMOUNT} for ${i} validator"

  if [[ -z "${RANDOM_SEED}" ]]; then
    velor genesis generate-keys --output-dir $user_dir
  else
    seed=$(printf "%064x" "$((${RANDOM_SEED_IN_DECIMAL} + i))")
    echo "seed=$seed for ${i}th validator"
    velor genesis generate-keys --random-seed $seed --output-dir $user_dir
  fi

  velor genesis set-validator-configuration --owner-public-identity-file $user_dir/public-keys.yaml --local-repository-dir $WORKSPACE \
    --username $username \
    --validator-host $validator_host \
    --full-node-host $fullnode_host \
    --stake-amount $CUR_STAKE_AMOUNT
done

# get the framework
# this is the directory the velor-framework is located in the velorlabs/tools docker image
cp $MOVE_FRAMEWORK_DIR/head.mrb ${WORKSPACE}/framework.mrb

# run genesis
velor genesis generate-genesis --local-repository-dir ${WORKSPACE} --output-dir ${WORKSPACE}

# delete all fullnode storage except for those from this era
kubectl get pvc -o name | grep /fn- | grep -v "e${ERA}-" | xargs -r kubectl delete
# delete all genesis secrets except for those from this era
kubectl get secret -o name | grep "genesis-e" | grep -v "e${ERA}-" | xargs -r kubectl delete

upload_genesis_blob() {
  if [ -z ${GENESIS_BLOB_UPLOAD_URL} ]; then
    echo "Skipping genesis blob upload, GENESIS_BLOB_UPLOAD_URL is not set"
    return 1
  fi

  local genesis_blob_path="${WORKSPACE}/genesis.blob"
  local signed_url status_code
  local genesis_blob_upload_url="${GENESIS_BLOB_UPLOAD_URL}"
  genesis_blob_upload_url="$genesis_blob_upload_url&namespace=${NAMESPACE}&method=PUT"

  # Set up a trap to remove the temporary file when the script exits
  local temp_file="$(mktemp)"
  trap 'rm -f "$temp_file"' EXIT

  # Get the signed URL for uploading the genesis.blob
  status_code=$(curl -s -o "$temp_file" -w "%{http_code}" "$genesis_blob_upload_url")

  if [[ "${status_code:0:1}" != "2" ]]; then
    echo "Failed to get signed URL, server responded with status code $status_code"
    return 1
  fi

  set +x
  signed_url=$(< "$temp_file")
  set -x

  # Upload the genesis.blob using the signed URL
  status_code=$(curl -s -o "$temp_file" -w "%{http_code}" -X PUT -T "$genesis_blob_path" "$signed_url")

  if [[ "${status_code:0:1}" != "2" ]]; then
    echo "Upload failed, server responded with status code $status_code"
    return 1
  fi

  echo "Upload successful"
  return 0
}

create_secrets() {
  local include_genesis_blob=$1

  for i in $(seq 0 $((NUM_VALIDATORS - 1))); do
    local username="${USERNAME_PREFIX}-${i}"
    local user_dir="${WORKSPACE}/${username}"

    local -a files_to_include=(
      "--from-file=waypoint.txt=${WORKSPACE}/waypoint.txt"
      "--from-file=validator-identity.yaml=${user_dir}/validator-identity.yaml"
      "--from-file=validator-full-node-identity.yaml=${user_dir}/validator-full-node-identity.yaml"
    )

    if [[ "$include_genesis_blob" == "true" ]]; then
      files_to_include+=("--from-file=genesis.blob=${WORKSPACE}/genesis.blob")
    fi

    kubectl create secret generic "${username}-genesis-e${ERA}" "${files_to_include[@]}"
  done
}

# Include the genesis blob in the secrets if we can't upload it
if upload_genesis_blob; then
  echo "Genesis blob uploaded successfully"
  create_secrets false
else
  echo "Genesis blob upload failed, including it in the secrets"
  create_secrets true
fi
