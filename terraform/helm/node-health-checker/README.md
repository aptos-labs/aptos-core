Aptos Node Health Checker Deployment
================================

This Helm chart deploys an instance of the Aptos Node Health Checker. NHC can be configured to run a comprehensive suite of health checks based on metrics, the API, and specific node "microservices", such as state sync, consensus, TPS, latency, etc.

Configuration
-------------

See [values.yaml]() for the full list of options you can configure.

Deployment
----------

1. Install Helm v3: https://helm.sh/docs/intro/install/
2. Configure `kubectl` with the Kubernetes cluster you wish to use.
3. Install the release, setting any options:

       $ helm install node-health-checker --set storage.class=gp2 .

[REST API]: https://github.com/aptos-labs/aptos-core/blob/main/api/doc/openapi.yaml
[values.yaml]: values.yaml
[Aptos DockerHub]: https://hub.docker.com/r/aptoslabs/node-checker/tags?page=1&ordering=last_updated
