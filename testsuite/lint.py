from forge import LocalShell, envoption, LocalFilesystem

import re
from typing import Optional, Tuple

import click



@click.group()
def main() -> None:
    pass


@main.command()
@envoption("GITHUB_EVENT_PATH")
@click.argument("paths", nargs=-1)
def helm(github_event_path: Optional[str], paths: Tuple[str]) -> None:
    filesystem = LocalFilesystem()

    if github_event_path:
        import json
        event = json.loads(filesystem.read(github_event_path))
        print(json.dumps(event, indent=2))

    shell = LocalShell()
    error = False
    for path in paths:
        result = shell.run(["helm", "lint", path])
        for line in result.output.decode().splitlines():
            if line.startswith("[ERROR]"):
                match = re.match(r".ERROR. (?P<section>[^:]+?): (?P<error_type>.*) at [(](?P<filename>.*):(?P<line>\d+)[)]: (?P<message>.*)", line)
                if match:
                    print("::error file={filename},line={line},col=1::{message}".format(**match.groupdict()))
                    error = True

    if error:
        raise SystemExit(1)



if __name__ == "__main__":
    main()