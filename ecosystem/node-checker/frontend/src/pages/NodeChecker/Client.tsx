import {EvaluationSummary, NodeCheckerClient} from "aptos-node-checker-client";
import {GlobalState} from "../../GlobalState";

export const DEFAULT_NHC_INSTANCE =
  "https://node-checker.prod.gcp.aptosdev.com";

export const NHC_INSTANCE_OVERRIDES = {
  local: "http://127.0.0.1:20121",
};

export type NhcInstanceOverride = keyof typeof NHC_INSTANCE_OVERRIDES;

export function determineNhcUrl(state: GlobalState) {
  if (state.network_name in NHC_INSTANCE_OVERRIDES) {
    return NHC_INSTANCE_OVERRIDES[state.network_name as NhcInstanceOverride];
  }
  return DEFAULT_NHC_INSTANCE;
}

function getClient(url: string) {
  return new NodeCheckerClient({
    BASE: url,
  });
}

export async function checkNode({
  nhcUrl,
  nodeUrl,
  baselineConfigurationName,
  apiPort,
  noisePort,
  publicKey,
}: {
  nhcUrl: string;
  nodeUrl: string;
  baselineConfigurationName?: string | undefined;
  apiPort?: number | undefined;
  noisePort?: number | undefined;
  publicKey?: string | undefined;
}): Promise<EvaluationSummary> {
  const client = getClient(nhcUrl);
  return client.default.getCheckNode({
    nodeUrl,
    baselineConfigurationName,
    apiPort,
    noisePort,
    publicKey,
  });
}

export interface MinimalConfiguration {
  name: string;
  prettyName: string;
  evaluators: string[];
}

// Return map of key to a minimal description of the configuration.
export async function getConfigurations({
  nhcUrl,
}: {
  nhcUrl: string;
}): Promise<Map<string, MinimalConfiguration>> {
  const client = getClient(nhcUrl);
  let configurations = await client.default.getGetConfigurations();
  configurations.sort((a, b) =>
    b.configuration_name.localeCompare(a.configuration_name),
  );
  let out = new Map<string, MinimalConfiguration>();
  for (const configuration of configurations) {
    out.set(configuration.configuration_name, {
      name: configuration.configuration_name,
      prettyName: configuration.configuration_name_pretty,
      evaluators: configuration.evaluators,
    });
  }
  return out;
}
