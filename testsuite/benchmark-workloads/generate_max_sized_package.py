import os
import subprocess
import random
import string
from pathlib import Path

MODULE_TEMPLATE = """
module {addr}::{module_name} {{

{use_stmt}

{structs}

{functions}

{entry}
}}
"""

STRUCT_TEMPLATE = """
    struct {name} {{
        {fields}
    }}
"""

FUNCTION_TEMPLATE = """
    public fun {name}() {{
        {body}
    }}
"""

ENTRY_TEMPLATE = """
    public entry fun main() {{
        {body}
    }}
"""

ADDRESS = "0xABCD"
PACKAGE_ROOT = "generated_package"
PACKAGE_NAME = "big_package"
MAX_PACKAGE_SIZE = 2_400_000  # 1.2 MB
MAX_MODULES = 768
INITIAL_IDENTIFIER_LENGTH = 20


def random_identifier(prefix: str = "id", length: int = INITIAL_IDENTIFIER_LENGTH) -> str:
    return prefix + "_" + ''.join(random.choices(string.ascii_letters + string.digits, k=length))


def generate_module(index: int, struct_count: int, field_count: int, function_count: int, body_lines: int, identifier_length: int, dependency_name: str | None) -> tuple[str, str]:
    module_name = "M_" + ''.join(random.choices(string.ascii_letters + string.digits, k=200))
    structs = []
    for _ in range(struct_count):
        struct_name = random_identifier("S", identifier_length)
        fields = ",\n        ".join([
            f"{random_identifier('f', identifier_length)}: u64"
            for _ in range(field_count)
        ])
        structs.append(STRUCT_TEMPLATE.format(name=struct_name, fields=fields))

    functions = []
    for _ in range(function_count):
        func_name = random_identifier("f", identifier_length)
        body = "\n        ".join([
            f"let {random_identifier('_x', identifier_length)} = 0u64;"
            for _ in range(body_lines)
        ])
        functions.append(FUNCTION_TEMPLATE.format(name=func_name, body=body))

    use_stmt = f"use {ADDRESS}::{dependency_name};" if dependency_name else ""
    entry = ENTRY_TEMPLATE.format(body=f"{dependency_name}::main();") if dependency_name else ENTRY_TEMPLATE.format(body="// end of chain")

    m = MODULE_TEMPLATE.format(
        addr=ADDRESS,
        module_name=module_name,
        use_stmt=use_stmt,
        structs="\n".join(structs),
        functions="\n".join(functions),
        entry=entry
    )
    return m, module_name


def write_package(modules: list[str]):
    src_path = Path(PACKAGE_ROOT) / "sources"
    os.makedirs(src_path, exist_ok=True)

    for i, mod in enumerate(modules):
        with open(src_path / f"module_{i}.move", "w") as f:
            f.write(mod)

    # Create a Move.toml
    with open(Path(PACKAGE_ROOT) / "Move.toml", "w") as f:
        f.write(f"""[package]
name = "{PACKAGE_NAME}"
version = "0.0.1"
""")


def build_package() -> bool:
    try:
        subprocess.run([
            "aptos",
            "move",
            "compile",
            f"--package-dir={PACKAGE_ROOT}"
        ], check=True, capture_output=True)
        return True
    except subprocess.CalledProcessError as e:
        print("Build failed:", e.stderr.decode())
        return False


def measure_mv_size() -> int:
    build_dir = Path(PACKAGE_ROOT) / "build" / PACKAGE_NAME / "bytecode_modules"
    total_size = 0
    if build_dir.exists():
        for path in build_dir.glob("*.mv"):
            total_size += os.path.getsize(path)
    return total_size


def generate_max_sized_package():
    module_names = []  # Ensure visibility inside generate_module
    # Increased initial parameters for large modules
    modules = []
    struct_count = 15
    field_count = 40
    function_count = 20
    body_lines = 100
    identifier_length = 150

    for i in range(MAX_MODULES):
        dependency_name = module_names[-1] if module_names else None
        mod, name = generate_module(i, struct_count, field_count, function_count, body_lines, identifier_length, dependency_name)
        module_names.append(name)
        modules.append(mod)
        write_package(modules)

        if not build_package():
            print("Build failed. Removing last module.")
            modules.pop()
            break

        size = measure_mv_size()
        print(f"Generated {i+1} modules, .mv size = {size} bytes")

        if size > MAX_PACKAGE_SIZE:
            print(".mv size limit exceeded. Removing last module.")
            modules.pop()
            break

        # Gradually increase complexity if we are within limits
        if (i + 1) % 10 == 0:
            struct_count += 1
            function_count += 1
            field_count += 2
            body_lines += 5
            identifier_length = min(255, identifier_length + 5)

    print(f"Final module count: {len(modules)}")
    write_package(modules)


if __name__ == "__main__":
    generate_max_sized_package()
