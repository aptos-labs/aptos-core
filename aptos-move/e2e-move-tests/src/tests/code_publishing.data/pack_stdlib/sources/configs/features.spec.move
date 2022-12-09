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

    spec set {
        pragma opaque;
    }

    spec contains {
        pragma opaque;
    }

    spec is_enabled(feature: u64): bool {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_is_enabled(feature);
    }

    spec fun spec_is_enabled(feature: u64): bool;
}
