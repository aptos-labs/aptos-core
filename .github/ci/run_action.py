"""Entry point for trusted Python actions: python3 -I run_action.py <command>.

-I keeps PYTHON* variables and the working directory (which can hold PR files)
out of the import path, so this file adds its own directory explicitly."""

import importlib
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from ci_actions.github import run_action  # noqa: E402

# command -> "module:function" inside ci_actions
COMMANDS: dict[str, str] = {
    "compute-authorized": "authorization:main",
    "pr-ci-report": "pr_ci_report:main",
    "indexer-processor-dispatch": "indexer_dispatch:main",
    "docker-capability-plan": "docker_plan:plan_main",
    "docker-status-plan": "docker_plan:status_main",
    "docker-forge-pr-report": "forge_report:main",
}


def main(argv: list[str]) -> int:
    if len(argv) != 2 or argv[1] not in COMMANDS:
        print(f"usage: run_action.py {{{'|'.join(sorted(COMMANDS))}}}", file=sys.stderr)
        return 2
    module_name, function_name = COMMANDS[argv[1]].split(":")
    return run_action(
        argv[1],
        lambda: getattr(importlib.import_module(f"ci_actions.{module_name}"), function_name)(),
    )


if __name__ == "__main__":
    sys.exit(main(sys.argv))
