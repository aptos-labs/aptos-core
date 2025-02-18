spec supra_framework::aggregator_factory {
    use supra_framework::aggregator;
    /// <high-level-req>
    /// No.: 1
    /// Requirement: During the module's initialization, it guarantees that the Supra framework is the caller and that the
    /// AggregatorFactory resource will move under the Supra framework account.
    /// Criticality: High
    /// Implementation: The initialize function is responsible for establishing the initial state of the module by
    /// creating the AggregatorFactory resource, indicating its presence within the module's context. Subsequently, the
    /// resource transfers to the Supra framework account.
    /// Enforcement: Formally verified via [high-level-req-1](initialize_aggregator_factory).
    ///
    /// No.: 2
    /// Requirement: To create a new aggregator instance, the aggregator factory must already be initialized and exist
    /// under the Supra account.
    /// Criticality: High
    /// Implementation: The create_aggregator_internal function asserts that AggregatorFactory exists for the Supra
    /// account.
    /// Enforcement: Formally verified via [high-level-req-2](CreateAggregatorInternalAbortsIf).
    ///
    /// No.: 3
    /// Requirement: Only the Supra framework address may create an aggregator instance currently.
    /// Criticality: Low
    /// Implementation: The create_aggregator function ensures that the address calling it is the Supra framework
    /// address.
    /// Enforcement: Formally verified via [high-level-req-3](create_aggregator).
    ///
    /// No.: 4
    /// Requirement: The creation of new aggregators should be done correctly.
    /// Criticality: High
    /// Implementation: The native new_aggregator function correctly creates a new aggregator.
    /// Enforcement: The new_aggregator native function has been manually audited.
    /// </high-level-req>
    ///
    spec module {
        pragma aborts_if_is_strict;
    }

    spec new_aggregator(aggregator_factory: &mut AggregatorFactory, limit: u128): Aggregator {
        pragma opaque;
        aborts_if false;
        ensures result == spec_new_aggregator(limit);
        ensures aggregator::spec_get_limit(result) == limit;
    }

    /// Make sure the caller is @supra_framework.
    /// AggregatorFactory is not under the caller before creating the resource.
    spec initialize_aggregator_factory(supra_framework: &signer) {
        use std::signer;
        let addr = signer::address_of(supra_framework);
        aborts_if addr != @supra_framework;
        aborts_if exists<AggregatorFactory>(addr);
        /// [high-level-req-1]
        ensures exists<AggregatorFactory>(addr);
    }

    spec create_aggregator_internal(limit: u128): Aggregator {
        /// [high-level-req-2]
        include CreateAggregatorInternalAbortsIf;
        ensures aggregator::spec_get_limit(result) == limit;
        ensures aggregator::spec_aggregator_get_val(result) == 0;
    }
    spec schema CreateAggregatorInternalAbortsIf {
        aborts_if !exists<AggregatorFactory>(@supra_framework);
    }

    /// Make sure the caller is @supra_framework.
    /// AggregatorFactory existed under the @supra_framework when Creating a new aggregator.
    spec create_aggregator(account: &signer, limit: u128): Aggregator {
        use std::signer;
        let addr = signer::address_of(account);
        /// [high-level-req-3]
        aborts_if addr != @supra_framework;
        aborts_if !exists<AggregatorFactory>(@supra_framework);
    }

    spec native fun spec_new_aggregator(limit: u128): Aggregator;

}
