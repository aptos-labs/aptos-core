import * as pulumi from '@pulumi/pulumi';
import * as azure from "@pulumi/azure-native";

import {
    location,
    era,
    chainId,
    chainName,
    validatorName,
    imageTag,
    zoneName,
    zoneResourceGroup,
    recordName,
    helmChart,
    helmValues,
    helmValuesFile,
    k8sApiSources,
    nodePoolSizes,
    k8sViewerGroups,
    k8sDebuggerGroups,
    utilityInstanceType,
    utilityInstanceNum,
    validatorInstanceType,
    validatorInstanceNum,
    validatorInstanceEnableTaint,
    enableLogger,
    loggerHelmValues,
    enableMonitoring,
    monitoringHelmValues,
    vnetAddress,
    workspaceName,
    k8sAdminGroups
} from "./config";

import { Auth } from "./auth";
import { Cluster } from "./cluster";
import { DNS } from "./dns";
import { Network } from "./network";
import { Kubernetes } from "./kubernetes";

export class AptosNodeAzure extends pulumi.ComponentResource {

    public readonly validatorEndpoint: pulumi.Output<string> | undefined;
    public readonly fullnodeEndpoint: pulumi.Output<string> | undefined;
    public readonly kubeConfig: pulumi.Output<string> | undefined;

    constructor(name: string, opts?: pulumi.ComponentResourceOptions) {
        super("aptos-node:azure:AptosNodeAzure", name, opts);

        const options = {
            parent: this
        };

        const resourceGroup = new azure.resources.ResourceGroup("validator", {
            location: location,
        }, options);

        const network = new Network("validator", {
            location: location,
            resourceGroupName: resourceGroup.name,
            vnetAddress: vnetAddress,
            workspaceName: workspaceName,
        }, options);

        const auth = new Auth("validator", {
            location: location,
            resourceGroupName: resourceGroup.name,
            subnetId: network.nodesSubnetId,
            workspaceName: workspaceName,
        }, options);

        const cluster = new Cluster("validator", {
            location: location,
            k8sApiSources: k8sApiSources,
            natIpAddress: network.natIpAddress,
            resourceGroupName: resourceGroup.name,
            servicePrincipalClientId: auth.applicationlId,
            servicePrincipalSecret: auth.servicePrincipalPassword,
            subnetId: network.nodesSubnetId,
            subnetPrefixes: network.nodesSubnetAddressPrefixes,
            subnetNetworkSecurityGroupName: network.nodesSecurityGroupName,
            utilityIntstanceType: utilityInstanceType,
            utilitiesNodePoolCount: nodePoolSizes["utilities"] || utilityInstanceNum,
            validatorInstanceEnableTaint: validatorInstanceEnableTaint,
            validatorsNodePoolCount: nodePoolSizes["validators"] || validatorInstanceNum,
            validatorInstanceType: validatorInstanceType,
            workspaceName: workspaceName,
            k8sAdminGroups: k8sAdminGroups,
        }, options);

        const kubernetes = new Kubernetes("validator", {
            kubeconfig: cluster.kubeConfig,
            imageTag: imageTag,
            era: era,
            chainId: chainId,
            chainName: chainName,
            validatorName: validatorName,
            workspaceName: workspaceName,
            validatorNodePoolName: cluster.validatorNodePoolName,
            enableLogging: enableLogger,
            enableMonitoring: enableMonitoring,
            helmChartPath: helmChart,
            helmValues: helmValues,
            helmValuesFile: helmValuesFile,
            loggerHelmValues: loggerHelmValues,
            monitoringHelmValues: monitoringHelmValues,
            k8sDebuggerGroups: k8sDebuggerGroups,
            k8sViewerGroups: k8sViewerGroups,
        }, options);

        if (zoneName && zoneName != "") {
            const dns = new DNS("validator", {
                workspaceName: workspaceName,
                zoneName: zoneName,
                zoneResourceGroup: zoneResourceGroup,
                recordName: recordName,
                validatorLoadBalancerIngress: kubernetes.validatorLoadBalancerIngress,
                fullnodeLoadBalancerIngress: kubernetes.fullnodeLoadBalancerIngress,
            }, options);

            this.validatorEndpoint = dns.validatorEndpoint;
            this.fullnodeEndpoint = dns.fullnodeEndpoint;
        }

        this.kubeConfig = cluster.kubeConfig;
    }
}