/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $NodeConfiguration = {
    properties: {
        node_address: {
            type: 'NodeAddress',
            isRequired: true,
        },
        configuration_name: {
            type: 'string',
            description: `This is the name we expect clients to send over the wire to select
            which configuration they want to use. e.g. devnet_fullnode`,
            isRequired: true,
        },
        configuration_name_pretty: {
            type: 'string',
            description: `This is the name we will show for this configuration to users.
            For example, if someone opens the NHC frontend, they will see this name
            in a dropdown list of configurations they can test their node against.
            e.g. "Devnet FullNode", "Testnet Validator Node", etc.`,
            isRequired: true,
        },
        evaluators: {
            type: 'array',
            contains: {
                type: 'string',
            },
            isRequired: true,
        },
        evaluator_args: {
            type: 'EvaluatorArgs',
            isRequired: true,
        },
        runner_args: {
            type: 'RunnerArgs',
            isRequired: true,
        },
    },
} as const;
