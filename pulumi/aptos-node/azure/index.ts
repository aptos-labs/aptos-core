import { AptosNodeAzure } from "./aptosNodeAzure";

const aptosNodeAzure = new AptosNodeAzure("aptos-node-azure");

export const validatorEndpoint = aptosNodeAzure.validatorEndpoint;
export const fullnodeEndpoint = aptosNodeAzure.fullnodeEndpoint;
export const kubeconfig = aptosNodeAzure.kubeConfig;