#!/bin/bash

set -e

K8S_CLUSTER=mycluster vector validate --no-environment ./files/vector-config.yaml ./files/vector-transforms.yaml
