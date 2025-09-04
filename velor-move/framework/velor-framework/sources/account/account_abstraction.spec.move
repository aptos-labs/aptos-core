spec velor_framework::account_abstraction {
    spec module {
        pragma verify = false;
    }


    spec fun spec_dispatchable_authenticate(
        account: signer,
        signing_data: AbstractionAuthData,
        function: &FunctionInfo
    ): signer;

    spec dispatchable_authenticate(account: signer, signing_data: AbstractionAuthData, function: &FunctionInfo): signer {
        pragma opaque;
        ensures [abstract] result == spec_dispatchable_authenticate(account, signing_data, function);
    }
}
