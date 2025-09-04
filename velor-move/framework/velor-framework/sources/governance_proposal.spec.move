spec velor_framework::governance_proposal {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: Creating a proposal should never abort but should always return a governance proposal resource.
    /// Criticality: Medium
    /// Implementation: Both create_proposal and create_empty_proposal functions return a GovernanceProposal resource.
    /// Enforcement: Enforced via [high-level-req-1.1](create_proposal) and [high-level-req-1.2](create_empty_proposal).
    ///
    /// No.: 2
    /// Requirement: The governance proposal module should only be accessible to the velor governance.
    /// Criticality: Medium
    /// Implementation: Both create_proposal and create_empty_proposal functions are only available to the friend module
    /// velor_framework::velor_governance.
    /// Enforcement: Enforced via friend module relationship.
    /// </high-level-req>
    ///
    spec create_proposal(): GovernanceProposal {
        aborts_if false;
        /// [high-level-req-1.1]
        ensures result == GovernanceProposal {};
    }

    spec create_empty_proposal(): GovernanceProposal {
        aborts_if false;
        /// [high-level-req-1.2]
        ensures result == GovernanceProposal {};
    }
}
