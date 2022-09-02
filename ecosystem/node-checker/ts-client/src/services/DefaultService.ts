/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { ConfigurationKey } from '../models/ConfigurationKey';
import type { EvaluationSummary } from '../models/EvaluationSummary';
import type { NodeConfiguration } from '../models/NodeConfiguration';

import type { CancelablePromise } from '../core/CancelablePromise';
import type { BaseHttpRequest } from '../core/BaseHttpRequest';

export class DefaultService {

    constructor(public readonly httpRequest: BaseHttpRequest) {}

    /**
     * Check the health of a given target node. You may specify a baseline
     * node configuration to use for the evaluation. If you don't specify
     * a baseline node configuration, we will attempt to determine the
     * appropriate baseline based on your target node.
     * @returns EvaluationSummary
     * @throws ApiError
     */
    public getCheckNode({
        nodeUrl,
        baselineConfigurationName,
        metricsPort = 9101,
        apiPort = 8080,
        noisePort = 6180,
        publicKey,
    }: {
        /**
         * The URL of the node to check. e.g. http://44.238.19.217 or http://fullnode.mysite.com
         */
        nodeUrl: string,
        /**
         * The name of the baseline node configuration to use for the evaluation, e.g. devnet_fullnode
         */
        baselineConfigurationName?: string,
        metricsPort?: number,
        apiPort?: number,
        noisePort?: number,
        /**
         * A public key for the node, e.g. 0x44fd1324c66371b4788af0b901c9eb8088781acb29e6b8b9c791d5d9838fbe1f.
         * This is only necessary for certain evaluators, e.g. HandshakeEvaluator.
         */
        publicKey?: string,
    }): CancelablePromise<EvaluationSummary> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/check_node',
            query: {
                'node_url': nodeUrl,
                'baseline_configuration_name': baselineConfigurationName,
                'metrics_port': metricsPort,
                'api_port': apiPort,
                'noise_port': noisePort,
                'public_key': publicKey,
            },
        });
    }

    /**
     * Check the health of the preconfigured node. If none was specified when
     * this instance of the node checker was started, this will return an error.
     * You may specify a baseline node configuration to use for the evaluation.
     * If you don't specify a baseline node configuration, we will attempt to
     * determine the appropriate baseline based on your target node.
     * @returns EvaluationSummary
     * @throws ApiError
     */
    public getCheckPreconfiguredNode({
        baselineConfigurationName,
    }: {
        baselineConfigurationName?: string,
    }): CancelablePromise<EvaluationSummary> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/check_preconfigured_node',
            query: {
                'baseline_configuration_name': baselineConfigurationName,
            },
        });
    }

    /**
     * Get the different baseline configurations the instance of NHC is
     * configured with. This method is best effort, it is infeasible to
     * derive (or even represent) some fields of the spec via OpenAPI,
     * so note that some fields will be missing from the response.
     * @returns NodeConfiguration
     * @throws ApiError
     */
    public getGetConfigurations(): CancelablePromise<Array<NodeConfiguration>> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/get_configurations',
        });
    }

    /**
     * Get just the keys and pretty names for the configurations, meaning
     * the configuration_name and configuration_name_pretty fields.
     * @returns ConfigurationKey
     * @throws ApiError
     */
    public getGetConfigurationKeys(): CancelablePromise<Array<ConfigurationKey>> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/get_configuration_keys',
        });
    }

}
