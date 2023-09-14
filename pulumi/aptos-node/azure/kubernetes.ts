import * as pulumi from '@pulumi/pulumi';
import * as k8s from '@pulumi/kubernetes';
import * as path from "path";
import * as random from "@pulumi/random";
import * as std from "@pulumi/std";
import { getSha1ForDirs, AptosLoadBalancerIngress } from "./utils"

export interface KubernetesConfig {
  kubeconfig: pulumi.Output<string>;
  imageTag: string;
  era: number;
  chainId: string;
  chainName: string;
  validatorName: string;
  workspaceName: string;
  validatorNodePoolName: pulumi.Output<string>;
  enableLogging: boolean;
  enableMonitoring: boolean;
  helmChartPath: string;
  helmValuesFile: string;
  helmValues: {};
  loggerHelmValues: {};
  monitoringHelmValues: {};
  k8sDebuggerGroups: string[];
  k8sViewerGroups: string[];
}

export class Kubernetes extends pulumi.ComponentResource {

  public readonly validatorLoadBalancerIngress: pulumi.Output<AptosLoadBalancerIngress>;
  public readonly fullnodeLoadBalancerIngress: pulumi.Output<AptosLoadBalancerIngress>;

  private readonly name: string;
  private readonly provider: k8s.Provider;
  constructor(name: string, args: KubernetesConfig, opts?: pulumi.ComponentResourceOptions) {
    super("aptos-node:azure:Kubernetes", name, {}, opts);

    this.name = name;
    this.provider = new k8s.Provider("kubernetes", {
      kubeconfig: args.kubeconfig,
    }, { parent: this });

    const clusterRole = new k8s.rbac.v1.ClusterRole(`${this.name}-debug`, {
      metadata: {
        name: "debug"
      },
      rules: [{
        apiGroups: [""],
        resources: ["pods/portforward", "pods/exec"],
        verbs: ["create"],
      }],
    }, { provider: this.provider, parent: this });

    const subjects = args.k8sDebuggerGroups.map(group => { return { kind: "Group", name: group } });
    if (args.k8sDebuggerGroups.length > 0) {
      new k8s.rbac.v1.ClusterRoleBinding(`${this.name}-aad-debuggers`, {
        roleRef: {
          apiGroup: "rbac.authorization.k8s.io",
          kind: "ClusterRole",
          name: clusterRole.metadata.name,
        },
        subjects: subjects,
      }, { provider: this.provider, parent: this });
    }

    if (args.k8sViewerGroups.length > 0 || args.k8sDebuggerGroups.length > 0) {

      // join views w/ debuggers to get a sum of both
      subjects.concat(args.k8sViewerGroups.map(group => { return { kind: "Group", name: group } }));
      new k8s.rbac.v1.ClusterRoleBinding(`${this.name}-aad-viewers`, {
        roleRef: {
          apiGroup: "rbac.authorization.k8s.io",
          kind: "ClusterRole",
          name: "view",
        },
        subjects: subjects,
      }, { provider: this.provider, parent: this });
    }

    const aptosNodeHelmChartPath = args.helmChartPath || path.join("..", "..", "..", "terraform", "helm", "aptos-node");

    // use the sha1 from the target chart directory to ensure we only update when the chart changes
    const validatorChartChangeTrigger = new random.RandomString(
      `${name}-changeTrigger`,
      {
        keepers: {
          chart_sha1: std.sha1Output({
            input: std.joinOutput({
              separator: "",
              input: getSha1ForDirs(aptosNodeHelmChartPath),
            }).result,
          }).result,
        },
        length: 12,
      },
      { parent: this },
    );

    const validatorRelease = new k8s.helm.v3.Release(`${this.name}-validator`, {
      name: args.workspaceName,
      createNamespace: true,
      maxHistory: 5,
      chart: aptosNodeHelmChartPath,
      values: {
        imageTag: args.imageTag,
        chain: {
          era: args.era,
          chainId: args.chainId,
          name: args.chainName,
        },
        validator: {
          name: args.validatorName,
          storage: {
            "class": "managed-premium",
          },
          nodeSelector: {
            agentpool: args.validatorNodePoolName,
          },
          tolerations: [{
            key: "aptos.org/nodepool",
            value: args.validatorNodePoolName,
            effect: "NoExecute",
          }],
        },
        fullnode: {
          storage: {
            "class": "managed-premium",
          },
          nodeSelector: {
            agentpool: args.validatorNodePoolName,
          },
          tolerations: [{
            key: "aptos.org/nodepool",
            value: args.validatorNodePoolName,
            effect: "NoExecute",
          }],
        },
        ...args.helmValues,
      },
      valueYamlFiles: args.helmValuesFile ? [new pulumi.asset.FileAsset(args.helmValuesFile)] : undefined,
    }, { provider: this.provider, parent: this, dependsOn: [validatorChartChangeTrigger] });


    if (args.enableLogging) {
      this.createLoggerHelmRelease(args.workspaceName, args.chainName, args.loggerHelmValues);
    }

    if (args.enableMonitoring) {
      this.createMonitoringHelmRelease(args.workspaceName, args.chainName, args.monitoringHelmValues, args.validatorName)
    }

    const validatorService = k8s.core.v1.Service.get("validator", pulumi.interpolate`${validatorRelease.status.namespace}/${args.workspaceName}-aptos-node-0-validator-lb`, { provider: this.provider, parent: this });
    this.validatorLoadBalancerIngress = validatorService.status.loadBalancer.apply(lb => this.retrieveIngressLoadBalancer(lb));

    const fullnodeService = k8s.core.v1.Service.get("fullnode", pulumi.interpolate`${validatorRelease.status.namespace}/${args.workspaceName}-aptos-node-0-fullnode-lb`, { provider: this.provider, parent: this });
    this.fullnodeLoadBalancerIngress = fullnodeService.status.loadBalancer.apply(lb => this.retrieveIngressLoadBalancer(lb));
  }

  retrieveIngressLoadBalancer(lb: k8s.types.output.core.v1.LoadBalancerStatus) {
    if (!lb) {
      return {
        ip: "",
        hostname: "",
        port: 0,
      };
    }

    return {
      ip: lb!.ingress![0].ip,
      hostname: lb!.ingress![0].hostname,
      port: lb!.ingress[0].ports[0].port,
    }
  }

  createLoggerHelmRelease(workspaceName: string, chainName: string, loggerHelmValues: {}) {
    const loggerHelmChartPath = path.join("..", "..", "..", "terraform", "lib", "helm", "logger");

    // use the sha1 from the target chart directory to ensure we only update when the chart changes
    const loggerChartChangeTrigger = new random.RandomString(
      `${this.name}-changeTrigger`,
      {
        keepers: {
          chart_sha1: std.sha1Output({
            input: std.joinOutput({
              separator: "",
              input: getSha1ForDirs(loggerHelmChartPath),
            }).result,
          }).result,
        },
        length: 12,
      },
      { parent: this },
    );

    new k8s.helm.v3.Release(`${this.name}-logger`, {
      name: `${workspaceName}-log`,
      chart: loggerHelmChartPath,
      maxHistory: 10,
      skipAwait: true,
      values: {
        logger: {
          name: "aptos-logger",
        },
        chain: {
          name: chainName,
        },
        serviceAccount: {
          create: false,
          name: `${workspaceName}-aptos-node-validator`,
        },
        ...loggerHelmValues,
      }
    }, { provider: this.provider, parent: this, dependsOn: [loggerChartChangeTrigger] });
  }

  createMonitoringHelmRelease(workspaceName: string, chainName: string, monitoringHelmValues: {}, validatorName: string) {
    const monitoringHelmChartPath = path.join("..", "..", "..", "terraform", "lib", "helm", "monitoring");

    // use the sha1 from the target chart directory to ensure we only update when the chart changes
    const monitoringChartChangeTrigger = new random.RandomString(
      `${this.name}-changeTrigger`,
      {
        keepers: {
          chart_sha1: std.sha1Output({
            input: std.joinOutput({
              separator: "",
              input: getSha1ForDirs(monitoringHelmChartPath),
            }).result,
          }).result,
        },
        length: 12,
      },
      { parent: this },
    );

    new k8s.helm.v3.Release(`${this.name}-logger`, {
      name: `${workspaceName}-mon`,
      chart: monitoringHelmChartPath,
      maxHistory: 10,
      skipAwait: true,
      values: {
        chain: {
          name: chainName,
        },
        validator: {
          name: validatorName,
        },
        monitoring: {
          prometheus: {
            storage: {
              "class": "default",
            },
          },
        },
        ...monitoringHelmValues,
      }
    }, { provider: this.provider, parent: this, dependsOn: [monitoringChartChangeTrigger] });
  }
}