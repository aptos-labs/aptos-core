spec aptos_framework::governance_proposal {
    spec create_proposal(): GovernanceProposal {
        aborts_if false;
        ensures result == GovernanceProposal {};
    }

    spec create_empty_proposal(): GovernanceProposal {
        aborts_if false;
        ensures result == GovernanceProposal {};
    }
}
