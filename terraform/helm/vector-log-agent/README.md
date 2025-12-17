# Vector DaemonSet

This Helm chart deploys a k8s DaemonSet that collects ALL logs of a k8s cluster via [Vector](https://vector.dev/).
The logger then sends the logs to any destination [Vector Sink](https://vector.dev/docs/reference/configuration/sinks) of your choice.
We also provide some recommended values for the sink configuration of _some_ sinks.
This chart is relatively generic and contains very little Aptos specific logic, except for some minor transforms under files/vector-transforms.yaml.

## General instructions

1. Install Helm v3: https://helm.sh/docs/intro/install/
2. Create a `my-values.yaml` to configure your sink. (check ./example-values for example configs.)
3. Deploy it via `helm upgrade vector --install --namespace vector --create-namespace ./ --values my-values.yaml`

## Sink specific instructions

### [Humio](https://www.humio.com/) Sink

1. Create a humio ingest token.
2. Deploy via:

```bash
helm upgrade --install vector --namespace vector --create-namespace ./ --values ./example-values/humio-sink.yaml --set k8sCluster=<cluster_name> --set-string secretVars.humio-credentials.HUMIO_TOKEN="<humio_token"
```

## Values

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| image.pullPolicy | string | `"IfNotPresent"` |  |
| image.repository | string | `"timberio/vector"` |  |
| image.tag | string | `"0.34.X-distroless-libc"` |  |
| k8sCluster | string | `"my_blockchain_cluster"` | human readible name of the kubernetes cluster this is being deployed to. This will be added as field `k8s.cluster=<cluster_name>` into each log event |
| loggingSinks | object | `{}` | Choose any (you can choose multiple) logging sinks supported by vector as found here https://vector.dev/docs/reference/configuration/sinks/ |
| secretVars | object | `{}` | secret environment variables to pass to the deployment |

## Troubleshooting

- `kubectl exec -it <name_of_a_vector_pod> -- vector top`

## Development

The directory `testing` contains some sample data and utility scripts to run vector locally and transform some test data.
This is useful to iterate on the parser, especially the remap transforms and see the output without redeploying vector every time.

- local testing:
- `./testing/test-transforms.sh` - pipes test1.json and test2.json files to vector and prints the transformed output to stdout
- `./testing/validate.sh` - runs `vector validate` to statically verify the correctness of the configuration

4/10/2025 -- Rustie's pro tip: to add new test cases, you can pull logs directly from the source/k8s/groundcover, and export as JSON

- quick local rendering (bypassing terraform):

```bash
helm template --namespace vector ./ --values ./example-values/humio-sink.yaml --set k8sCluster=<cluster_name> --set-string secretVars.humio-credentials.HUMIO_TOKEN="<humio_token" > rendered.yaml
```

- quick deployment of the humio config (bypassing terraform):

```bash
helm upgrade --install vector --namespace vector --create-namespace ./ --values ./example-values/humio-sink.yaml --set k8sCluster=<cluster_name> --set-string secretVars.humio-credentials.HUMIO_TOKEN="<humio_token"
```
