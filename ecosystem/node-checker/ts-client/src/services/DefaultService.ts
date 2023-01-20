/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { CheckSummary } from '../models/CheckSummary';
import type { ConfigurationDescriptor } from '../models/ConfigurationDescriptor';

import type { CancelablePromise } from '../core/CancelablePromise';
import type { BaseHttpRequest } from '../core/BaseHttpRequest';

export class DefaultService {

    constructor(public readonly httpRequest: BaseHttpRequest) {}

    /**
     * Check the health of a given target node. You must specify a baseline
     * node configuration to use for the evaluation and the URL of your node,
     * without including any port or endpoints. All other parameters are optional.
     * For example, if your node's API port is open but the rest are closed, only
     * set the `api_port`.
     * @returns CheckSummary
     * @throws ApiError
     */
    public getCheck({
        baselineConfigurationId,
        nodeUrl,
        metricsPort,
        apiPort,
        noisePort,
        publicKey,
    }: {
        /**
         * The ID of the baseline node configuration to use for the evaluation, e.g. devnet_fullnode
         */
        baselineConfigurationId: string,
        /**
         * The URL of the node to check, e.g. http://44.238.19.217 or http://fullnode.mysite.com
         */
        nodeUrl: string,
        /**
         * If given, we will assume the metrics service is available at the given port.
         */
        metricsPort?: number,
        /**
         * If given, we will assume the API is available at the given port.
         */
        apiPort?: number,
        /**
         * If given, we will assume that clients can communicate with your node via noise at the given port.
         */
        noisePort?: number,
        /**
         * A public key for the node, e.g. 0x44fd1324c66371b4788af0b901c9eb8088781acb29e6b8b9c791d5d9838fbe1f.
         * This is only necessary for certain checkers, e.g. HandshakeChecker.
         */
        publicKey?: string,
    }): CancelablePromise<CheckSummary> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/check',
            query: {
                'baseline_configuration_id': baselineConfigurationId,
                'node_url': nodeUrl,
                'metrics_port': metricsPort,
                'api_port': apiPort,
                'noise_port': noisePort,
                'public_key': publicKey,
            },
        });
    }

    /**
     * Get the IDs and pretty names for the configurations. For example,
     * devnet_fullnode as the ID and "Devnet Fullnode Checker" as the
     * pretty name.
     * @returns ConfigurationDescriptor
     * @throws ApiError
     */
    public getConfigurations(): CancelablePromise<Array<ConfigurationDescriptor>> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/configurations',
        });
    }

}
