import { Network } from "./api-endpoints";

export const NODE_URL = process.env.APTOS_NODE_URL!;
export const APTOS_FAUCET_URL = process.env.APTOS_FAUCET_URL!;

export const DEFAULT_NETWORK = Network.TESTNET;
export const DEFAULT_FAUCET = Network.TESTNET;

export const DEFAULT_MAX_GAS_AMOUNT = 200000;
// Transaction expire timestamp
export const DEFAULT_TXN_EXP_SEC_FROM_NOW = 20;
// How long does SDK wait for txhn to finish
export const DEFAULT_TXN_TIMEOUT_SEC = 20;
