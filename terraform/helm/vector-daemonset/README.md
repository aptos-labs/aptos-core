Vector DaemonSet
===

This Helm chart deploys a k8s DaemonSet that collects ALL logs of a k8s cluster via [Vector](https://vector.dev/).
The logger then sends the logs to any destination [Vector Sink](https://vector.dev/docs/reference/configuration/sinks) of your choice.

We also provide some recommended values for the sink configuration of _some_ sinks.

## General instructions

1. Install Helm v3: https://helm.sh/docs/intro/install/
2. Create a `my-values.yaml` to configure your sink
3. Deploy it via `helm upgrade vector --install --namespace vector --create-namespace ./ --values my-values.yaml`


## Sink specific instructions

### [Humio](https://www.humio.com/) Sink

1. Create a humio ingest token and follow the instructions under [humio-sink.yaml](./example-values/humio-sink.yaml) to create a corresponding k8s Secret.
2. Deploy it via `helm upgrade vector --install --namespace vector --create-namespace --values ./example-values/humio-sink.yaml ./`


## Troubleshooting

- `kubectl exec -it <name_of_a_vector_pod> -- vector top`