import * as pulumi from "@pulumi/pulumi";
import * as azure from "@pulumi/azure-native";
import { AacAudioProfile } from "@pulumi/azure-native/media";

export interface ClusterArgs {
    k8sApiSources: string[];
    kubernetesVersion?: string;
    location: string;
    natIpAddress: pulumi.Output<string>;
    servicePrincipalClientId: pulumi.Output<string>;
    servicePrincipalSecret: pulumi.Output<string>;
    subnetId: pulumi.Output<string>;
    subnetNetworkSecurityGroupName: pulumi.Output<string>;
    subnetPrefixes: pulumi.Output<string[]>;
    resourceGroupName: pulumi.Output<string>;
    utilityIntstanceType: string;
    validatorInstanceEnableTaint: boolean;
    validatorsNodePoolCount: number;
    utilitiesNodePoolCount: number;
    validatorInstanceType: string;
    workspaceName: string;
    k8sAdminGroups: string[];
}

export class Cluster extends pulumi.ComponentResource {
    public readonly kubeConfig: pulumi.Output<string>;
    public readonly validatorNodePoolName: pulumi.Output<string>;

    constructor(name: string, args: ClusterArgs, opts?: pulumi.ComponentResourceOptions) {
        super("aptos-node:azure:Cluster", name, args, opts);

        const k8sVersion = args.kubernetesVersion || "1.25.6";
        const authorizedRanges = args.natIpAddress.apply(ip => {
            return [
                ip,
                ...args.k8sApiSources
            ];
        });

        const options = {
            parent: this,
            deleteBeforeReplace: true,
        };

        const enableRbac = args.k8sAdminGroups.length > 0;
        const aadProfile = enableRbac ? {
            managed: true,
            adminGroupObjectIDs: args.k8sAdminGroups,
            tenantID: azure.authorization.getClientConfig().then(config => config.tenantId),
        } : {};

        const cluster = new azure.containerservice.ManagedCluster(`${name}-aks`, {
            resourceName: `aptos-${args.workspaceName}`,
            resourceGroupName: args.resourceGroupName,
            location: args.location,
            dnsPrefix: `aptos-${args.workspaceName}`,
            kubernetesVersion: k8sVersion,
            apiServerAccessProfile: {
                authorizedIPRanges: authorizedRanges,
            },
            networkProfile: {
                networkPlugin: "kubenet",
                networkPolicy: "calico",
                loadBalancerSku: "standard",
            },
            agentPoolProfiles: [
                {
                    name: "utilities",
                    orchestratorVersion: k8sVersion,
                    vmSize: args.utilityIntstanceType,
                    vnetSubnetID: args.subnetId,
                    count: args.utilitiesNodePoolCount,
                    osDiskSizeGB: 30,
                    mode: azure.containerservice.AgentPoolMode.System
                },
                {
                    name: "validators",
                    vmSize: args.validatorInstanceType,
                    vnetSubnetID: args.subnetId,
                    count: args.validatorsNodePoolCount,
                    osDiskSizeGB: 30,
                    nodeTaints: args.validatorInstanceEnableTaint ? ["node-role.kubernetes.io/validator=:NoExecute"] : [],
                    mode: azure.containerservice.AgentPoolMode.User,
                }
            ],
            servicePrincipalProfile: {
                clientId: args.servicePrincipalClientId,
                secret: args.servicePrincipalSecret,
            },
            enableRBAC: enableRbac,
            aadProfile: aadProfile,
        }, options);

        const workspace = new azure.operationalinsights.Workspace(`aptos-${args.workspaceName}`, {
            resourceGroupName: args.resourceGroupName,
            workspaceName: `aptos-${args.workspaceName}`,
            location: args.location,
            retentionInDays: 30,
        }, options);

        new azure.insights.DiagnosticSetting("cluster", {
            name: "cluster",
            resourceUri: cluster.id,
            workspaceId: workspace.id,
            logs: [
                {
                    enabled: true,
                    category: "kube-apiserver",
                },
                {
                    enabled: true,
                    category: "kube-controller-manager",
                },
                {
                    enabled: true,
                    category: "kube-scheduler",
                },
                {
                    enabled: true,
                    category: "kube-audit",
                },
                {
                    enabled: true,
                    category: "guard",
                },
            ]
        }, options);

        const sourceAddressPrefixes = pulumi.all([args.subnetPrefixes, cluster.networkProfile]).apply(([prefixes, profile]) => {
            return [
                ...prefixes,
                profile!.serviceCidr!,
                profile!.podCidr!
            ]
        });

        new azure.network.SecurityRule(`${name}-nodes-tcp`, {
            access: "Allow",
            destinationAddressPrefix: "*",
            destinationPortRange: "1025-65535",
            direction: "Inbound",
            name: "nodes-tcp",
            networkSecurityGroupName: args.subnetNetworkSecurityGroupName,
            priority: 1000,
            protocol: "Tcp",
            resourceGroupName: args.resourceGroupName,
            sourceAddressPrefixes: sourceAddressPrefixes,
            sourcePortRange: "*",
        }, options);

        new azure.network.SecurityRule(`${name}-nodes-udp`, {
            access: "Allow",
            destinationAddressPrefix: "*",
            destinationPortRange: "1025-65535",
            direction: "Inbound",
            name: "nodes-udp",
            networkSecurityGroupName: args.subnetNetworkSecurityGroupName,
            priority: 1010,
            protocol: "Udp",
            resourceGroupName: args.resourceGroupName,
            sourceAddressPrefixes: sourceAddressPrefixes,
            sourcePortRange: "*",
        }, options);

        new azure.network.SecurityRule(`${name}-nodes-icmp`, {
            name: "nodes-icmp",
            priority: 1020,
            direction: "Inbound",
            access: "Allow",
            protocol: "Icmp",
            destinationAddressPrefix: "*",
            destinationPortRange: "*",
            sourceAddressPrefixes: sourceAddressPrefixes,
            sourcePortRange: "*",
            networkSecurityGroupName: args.subnetNetworkSecurityGroupName,
            resourceGroupName: args.resourceGroupName,
        }, options);

        new azure.network.SecurityRule(`${name}-nodes-dns`, {
            name: "nodes-dns",
            priority: 1030,
            direction: "Inbound",
            access: "Allow",
            protocol: "Udp",
            destinationAddressPrefix: "*",
            destinationPortRange: "53",
            sourceAddressPrefixes: sourceAddressPrefixes,
            sourcePortRange: "*",
            networkSecurityGroupName: args.subnetNetworkSecurityGroupName,
            resourceGroupName: args.resourceGroupName,
        }, options);

        // retrieve the cluster credentials in order obtain a kubeconfig for the k8s provider
        const credentials = azure.containerservice.listManagedClusterAdminCredentialsOutput({
            resourceGroupName: args.resourceGroupName,
            resourceName: cluster.name
        });

        this.kubeConfig = credentials.kubeconfigs[0].value.apply((config) => Buffer.from(config, "base64").toString());

        this.validatorNodePoolName = cluster.agentPoolProfiles.apply(profiles => {
            const validatorProfile = profiles!.find(profile => profile.name === "validators");
            return validatorProfile!.name;
        });
    }
}