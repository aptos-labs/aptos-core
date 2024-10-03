# Forge K8s Deployer Backend

This backend manages Forge "deployers", which are k8s jobs that spin up the necessary k8s infrastructure for Forge tests to run.
They mostly involve state management of the Forge namespace, ancillary resources like configmaps, and the deployer jobs themselves.

Forge deployers:

- Each deploy a single "component" of Forge infra, which may be dependent on some other components or resources. For example, this can be an indexer stack, which in turn relies on a testnet stack to exist
- Can take in customization values via the env var FORGE_DEPLOY_VALUES_JSON
- Have a known values schema but mostly rely on a "profile" that is suitable for most tests, that contains default sane values

## Implementation Notes

Forge Deployers require access to create namespaces, SA, rolebindings, etc. and grant the `cluster-admin` clusterrole to the namespace it creates. As such, Forge should always be run in an isolated k8s cluster
