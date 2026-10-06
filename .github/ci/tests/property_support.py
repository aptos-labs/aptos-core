"""Bounded input strategies and reproducible profiles for CI contract tests."""

import os
import string

from hypothesis import settings, strategies as st


def configure_profiles():
    """Select a profile before constructing property-test decorators."""
    common = dict(database=None, deadline=None, print_blob=True, suppress_health_check=())
    settings.register_profile("ci", max_examples=100, derandomize=True, **common)
    settings.register_profile("explore", max_examples=1_000, derandomize=False, **common)
    profile = os.environ.get("HYPOTHESIS_PROFILE", "ci")
    if profile not in {"ci", "explore"}:
        raise ValueError(f"Unknown HYPOTHESIS_PROFILE: {profile!r}; expected ci or explore")
    settings.load_profile(profile)


SAFE_COMPONENT = st.text(alphabet=string.ascii_letters + string.digits + "_-", min_size=1, max_size=16)
REPOSITORY = st.tuples(SAFE_COMPONENT, SAFE_COMPONENT).map(lambda parts: "/".join(parts))
BRANCH = st.text(alphabet=string.ascii_letters + string.digits + "_./+-", min_size=1, max_size=64)
SHA = st.text(alphabet="0123456789abcdef", min_size=40, max_size=40)
POSITIVE_ID = st.integers(min_value=1, max_value=2**53 - 1)
IDENTITIES = st.fixed_dictionaries({
    "source_repository": REPOSITORY,
    "source_sha": SHA,
    "base_sha": SHA,
    "pr_number": st.integers(min_value=1, max_value=10**10 - 1).map(str),
    "run_id": POSITIVE_ID.map(str),
    "run_attempt": st.integers(min_value=1, max_value=999_999).map(str),
    "variant": st.just("release"),
    "artifact_repo": st.just("us-docker.pkg.dev/aptos-registry/docker"),
})
