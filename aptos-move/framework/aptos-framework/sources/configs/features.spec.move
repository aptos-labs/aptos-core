/// Maintains feature flags.
spec aptos_framework::features {
    spec module {
        pragma verify = false;
    }

    spec change_feature_flags {
        pragma opaque = true;
        modifies global<Features>(@aptos_framework);
    }

    spec code_dependency_check_enabled {
        pragma opaque = true;
    }

}
