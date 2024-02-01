module 0x1::governance_proposal {
    struct GovernanceProposal has drop, store {
        dummy_field: bool,
    }
    
    public(friend) fun create_empty_proposal() : GovernanceProposal {
        create_proposal()
    }
    
    public(friend) fun create_proposal() : GovernanceProposal {
        GovernanceProposal{dummy_field: false}
    }
    
    // decompiled from Move bytecode v6
}
