import os
from pathlib import Path


def classify_event(event_name: str, base_ref: str, ref_name: str) -> dict[str, bool]:
    if event_name not in {"pull_request", "push", "workflow_dispatch"}:
        raise ValueError(f"Unsupported Lint+Test event: {event_name!r}")

    branch = base_ref if event_name == "pull_request" else ref_name
    if not branch:
        raise ValueError(f"Missing branch for Lint+Test event: {event_name!r}")

    is_release = branch in {"devnet", "testnet", "mainnet"} or branch.startswith(
        ("aptos-release-v", "aptos-node-v")
    )
    development_pr = event_name == "pull_request" and not is_release
    return {
        "run_freshness": development_pr,
        "run_targeted_tests": development_pr,
        "run_full_tests": not development_pr,
    }


if __name__ == "__main__":
    decisions = classify_event(
        os.environ.get("CI_EVENT_NAME", ""),
        os.environ.get("CI_BASE_REF", ""),
        os.environ.get("CI_REF_NAME", ""),
    )
    output = "".join(
        f"{name}={str(enabled).lower()}\n" for name, enabled in decisions.items()
    )
    with Path(os.environ["GITHUB_OUTPUT"]).open("a") as output_file:
        output_file.write(output)
    print(output, end="")
