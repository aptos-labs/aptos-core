#!/bin/bash

set -e

# Using the local IP address is equivalent of looping over all the nodes
# since the test is executed locally on each node
IP="127.0.0.1"
# Faucet IP needs to be substituted with the actual IP address of the faucet
IP_FAUCET="<FAUCET_IP>"

aptos init --profile test1 --rest-url http://"${IP}":8080 --faucet-url http://"${IP_FAUCET}":8000 --assume-yes

