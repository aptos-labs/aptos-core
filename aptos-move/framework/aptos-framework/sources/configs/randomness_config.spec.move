spec aptos_framework::randomness_config {
    spec current {
        aborts_if false;
    }

    spec on_new_epoch() {
        include config_buffer::OnNewEpochAbortsIf<RandomnessConfig>;
    }
}
