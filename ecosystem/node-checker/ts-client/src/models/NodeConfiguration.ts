/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { EvaluatorArgs } from './EvaluatorArgs';
import type { NodeAddress } from './NodeAddress';
import type { RunnerArgs } from './RunnerArgs';

export type NodeConfiguration = {
    node_address: NodeAddress;
    /**
     * This is the name we expect clients to send over the wire to select
     * which configuration they want to use. e.g. devnet_fullnode
     */
    configuration_name: string;
    /**
     * This is the name we will show for this configuration to users.
     * For example, if someone opens the NHC frontend, they will see this name
     * in a dropdown list of configurations they can test their node against.
     * e.g. "Devnet FullNode", "Testnet Validator Node", etc.
     */
    configuration_name_pretty: string;
    /**
     * The evaluators to use, e.g. state_sync_version, consensus_proposals, etc.
     */
    evaluators: Array<string>;
    evaluator_args: EvaluatorArgs;
    runner_args: RunnerArgs;
};

