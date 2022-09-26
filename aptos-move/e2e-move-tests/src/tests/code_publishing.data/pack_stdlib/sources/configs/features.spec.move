/// Maintains feature flags.
spec std::features {
    spec module {
        pragma verify = false;
    }

    spec change_feature_flags {
        pragma opaque = true;
        modifies global<Features>(@std);
    }

    spec code_dependency_check_enabled {
        pragma opaque = true;
    }

}
