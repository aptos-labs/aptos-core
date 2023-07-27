#!/bin/bash

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)

# ensure kubecontext is set to kind-kind
kubectl config set-context kind-kind

# start nginx
kubectl get namespace | grep -q "^test" || kubectl create namespace test
kubectl apply -n test -f $SCRIPT_DIR/nginx.yaml

# wait for nginx to be ready
for i in {1..30}; do
  kubectl get event -n test
  kubectl wait --for=condition=ready pod -n test -l app=nginx --timeout=60s && break
  sleep 10
done
kubectl port-forward -n test svc/nginx 8031:80 >/dev/null 2>&1 &
port_forward_command_pid=$!

# test nginx by curling it
ret=1
for i in {1..30}; do
  curl localhost:8031
  ret=$?
  if [ $ret -eq 0 ]; then
    break
  fi
  sleep 10
done
if [ $ret -ne 0 ]; then
  echo "curl failed with exit code $ret"
  echo -e "\n\nTEST FAILED!!!\n"
else
  echo -e "\n\nTEST PASSED!!!\n"
fi

# clean up
echo "====="
echo "Resources still running:"
kubectl get all -n test
echo "====="
echo "To kill port-forward, run: $ kill ${port_forward_command_pid}"
echo "To delete test namespace, run: $ kubectl delete namespace test"

exit $ret
