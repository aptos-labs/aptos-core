import argparse
import copy
import json
import subprocess
import tarfile
import tempfile
from pathlib import Path

from verify_hosted_shards import selected_tests


def runtime_path(binary_path, target):
    return Path("target") / Path(binary_path).relative_to(target)


def package(source, inventories, output, strip_tool):
    output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="hosted-runtime-") as tmp:
        root = Path(tmp)
        subprocess.run(
            ["tar", "--zstd", "-xf", str(source.resolve()), "-C", str(root)],
            check=True,
        )
        metadata_path = root / "target/nextest/binaries-metadata.json"
        metadata = json.loads(metadata_path.read_text())
        target = metadata["rust-build-meta"]["target-directory"]
        binaries = metadata["rust-binaries"]
        binary_paths = {
            binary_id: runtime_path(binary["binary-path"], target)
            for binary_id, binary in binaries.items()
        }
        before = sum(p.stat().st_size for p in root.rglob("*") if p.is_file())
        stripped = 0
        for path in root.rglob("*"):
            if path.is_file() and not path.is_symlink():
                with path.open("rb") as stream:
                    elf = stream.read(4) == b"\x7fELF"
                if elf:
                    subprocess.run([strip_tool, "--strip-debug", str(path)], check=True)
                    stripped += 1
        after = sum(p.stat().st_size for p in root.rglob("*") if p.is_file())
        print(f"Runtime bytes before/after stripping: {before}/{after}; ELF files: {stripped}")
        sizes = []
        for shard, inventory_path in enumerate(inventories, 1):
            selected = selected_tests(json.loads(inventory_path.read_text()))
            ids = {binary_id for binary_id, _ in selected}
            if not ids <= binaries.keys():
                raise ValueError("Shard inventory contains unknown test binaries")
            shard_metadata = copy.deepcopy(metadata)
            shard_metadata["rust-binaries"] = {key: binaries[key] for key in ids}
            metadata_path.write_text(json.dumps(shard_metadata))
            excluded = {path for key, path in binary_paths.items() if key not in ids}
            archive_path = output / f"unit-tests-{shard}.tar.zst"
            with archive_path.open("wb") as destination:
                process = subprocess.Popen(
                    ["zstd", "-T4", "-6", "-c"], stdin=subprocess.PIPE, stdout=destination
                )
                try:
                    with tarfile.open(fileobj=process.stdin, mode="w|") as archive:
                        for path in sorted(root.rglob("*")):
                            relative = path.relative_to(root)
                            if relative not in excluded:
                                archive.add(path, arcname=str(relative), recursive=False)
                finally:
                    process.stdin.close()
                    status = process.wait()
                if status:
                    raise RuntimeError(f"Compression failed for shard {shard}")
            size = archive_path.stat().st_size
            sizes.append(size)
            print(f"Shard {shard}: {len(ids)} binaries, {len(selected)} tests, {size} bytes")
        return sizes


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("archive", type=Path)
    parser.add_argument("inventories", nargs=4, type=Path)
    parser.add_argument("--output", type=Path, default=Path("shard-archives"))
    parser.add_argument("--strip-tool", default="llvm-strip-22")
    args = parser.parse_args()
    package(args.archive, args.inventories, args.output, args.strip_tool)
