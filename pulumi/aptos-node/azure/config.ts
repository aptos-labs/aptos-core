import * as pulumi from "@pulumi/pulumi";

const config = new pulumi.Config();
const azureConfig = new pulumi.Config("azure-native");

// If specified, overrides the usage of Terraform workspace for naming purposes
const workspaceNameOverride = config.get("workspaceNameOverride") || "";
// TODO: The workspace name is used in the names of resources, so make sure
// the name matches the name of the TF workspace that was used to setup
// resources in the respective workspace.
export const workspaceName = workspaceNameOverride || pulumi.getStack();

// Azure region
export const location = azureConfig.require("location");

// Chain era, used to start a clean chain
export const era = config.getNumber("era") || 1;

// Aptos chain ID
export const chainId = config.get("chainId") || "TESTING";

// Aptos chain name
export const chainName = config.get("chainName") || "testnet";

// Name of the validator node ownernotImplemented("lookup(var.node_pool_sizes,\"utilities\",var.utility_instance_num)")
export const validatorName = config.require("validatorName");

// Docker image tag for Aptos node
export const imageTag = config.get("imageTag") || "devnet";

// Zone name of Azure DNS domain to create records in
export const zoneName = config.get("zoneName") || "";

// Azure resource group name of the DNS zone
export const zoneResourceGroup = config.get("zoneResourceGroup") || "";

// DNS record name to use (<workspace> is replaced with the TF workspace name)
export const recordName = config.get("recordName") || `${workspaceName}.aptos`;

// Path to aptos-validator Helm chart file
export const helmChart = config.get("helmChart") || "";

// Map of values to pass to Helm
export const helmValues = config.getObject("helmValues") || {};

// Path to file containing values for Helm chart
export const helmValuesFile = config.get("helmValuesFile") || "";

// List of CIDR subnets which can access the Kubernetes API endpoint
export const k8sApiSources = config.getObject<Array<string>>("k8sApiSources") || ["0.0.0.0/0"];

// List of AD Group IDs to configure as Kubernetes admins
export const k8sAdminGroups = config.getObject<Array<string>>("k8sAdminGroups") || [];

// Override the number of nodes in the specified pool
export const nodePoolSizes = config.getObject<Record<string, number>>("nodePoolSizes") || {};

// List of AD Group IDs to configure as Kubernetes viewers
export const k8sViewerGroups = config.getObject<Array<string>>("k8sViewerGroups") || [];

// List of AD Group IDs to configure as Kubernetes debuggers
export const k8sDebuggerGroups = config.getObject<Array<string>>("k8sDebuggerGroups") || [];

// Instance type used for utilities
export const utilityInstanceType = config.get("utilityInstanceType") || "Standard_B8ms";

// Number of instances for utilities
export const utilityInstanceNum = config.getNumber("utilityInstanceNum") || 1;

// Instance type used for validator and fullnodes
export const validatorInstanceType = config.get("validatorInstanceType") || "Standard_F4s_v2";

// Number of instances used for validator and fullnodes
export const validatorInstanceNum = config.getNumber("validatorInstanceNum") || 2;

// Whether to taint the instances in the validator nodegroup
export const validatorInstanceEnableTaint = config.getBoolean("validatorInstanceEnableTaint") || false;

// Enable logger helm chart
export const enableLogger = config.getBoolean("enableLogger") || false;

// Map of values to pass to logger Helm
export const loggerHelmValues = config.getObject("loggerHelmValues") || {};

// Enable monitoring helm chart
export const enableMonitoring = config.getBoolean("enableMonitoring") || false;

// Map of values to pass to monitoring Helm
export const monitoringHelmValues = config.getObject("monitoringHelmValues") || {};

export const vnetAddress = config.get("vnetAddress") || "19.168.0.0/16";
