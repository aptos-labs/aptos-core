#!/usr/bin/env python3

# Script to bump the LATEST_GAS_FEATURE_VERSION and create the next gas feature version entry
# if it does not exist yet.

import re, os

def get_file_path():
    """Get the absolute path to the ver.rs file, relative to the Python script."""
    script_dir = os.path.dirname(os.path.abspath(__file__))
    return os.path.join(script_dir, "src/ver.rs")

def read_file(filename):
    """Reads the entire content of the Rust file."""
    with open(filename, 'r') as f:
        return f.read()

def write_file(filename, content):
    """Writes the modified content back to the Rust file."""
    with open(filename, 'w') as f:
        f.write(content)

def get_latest_version(content):
    """Find the current LATEST_GAS_FEATURE_VERSION value."""
    match = re.search(r'LATEST_GAS_FEATURE_VERSION: u64 = gas_feature_versions::(RELEASE_V1_\d+);', content)
    if not match:
        raise ValueError("Could not find the LATEST_GAS_FEATURE_VERSION in the file.")
    return match.group(1)

def get_gas_versions_block(content):
    """Find the entire gas_feature_versions module block."""
    match = re.search(r'(pub mod gas_feature_versions \{.*?\})', content, re.DOTALL)
    if not match:
        raise ValueError("Could not find the gas_feature_versions block.")
    return match.group(1)

def get_all_versions(gas_versions_block):
    """Retrieve all existing RELEASE_V1_XX entries and their values from the block."""
    return re.findall(r'RELEASE_V1_(\d+): u64 = (\d+);', gas_versions_block)

def increment_version(latest_version, all_versions, gas_versions_block):
    """Increment the version, adding a new entry if necessary."""
    latest_num = int(latest_version.split('_')[-1])
    latest_value = max(int(value) for _, value in all_versions)

    # Check if there's already a RELEASE_V1_(latest_num + 1) entry
    next_num = latest_num + 1
    next_version = f"RELEASE_V1_{latest_num + 1}"

    for version, _ in all_versions:
        if next_num == int(version):
            return next_version, gas_versions_block, False  # No new entry needed

    # If not, create a new one with the next value
    new_value = latest_value + 1
    new_entry = f'    pub const {next_version}: u64 = {new_value};\n'

    # Insert the new entry just before the closing brace of the module
    modified_block = re.sub(r'(\}\s*)$', new_entry + r'\1', gas_versions_block)
    return next_version, modified_block, True  # New entry added

def update_gas_versions_block(content, new_block):
    """Replace the old gas_feature_versions block with the new modified one."""
    return re.sub(r'(pub mod gas_feature_versions \{.*?\})', new_block, content, flags=re.DOTALL)

def update_latest_version(content, new_version):
    """Update the LATEST_GAS_FEATURE_VERSION with the new version."""
    return re.sub(
        r'(LATEST_GAS_FEATURE_VERSION: u64 = gas_feature_versions::)RELEASE_V1_\d+;',
        rf'\1{new_version};',
        content
    )

def main():
    # Step 1: Read the file content
    file_path = get_file_path()

    content = read_file(file_path)

    # Step 2: Get the current latest version and the gas_feature_versions block
    latest_version = get_latest_version(content)
    gas_versions_block = get_gas_versions_block(content)

    # Step 3: Get all existing versions from the gas_feature_versions block
    all_versions = get_all_versions(gas_versions_block)

    # Step 4: Increment the version or create a new one if needed
    new_version, modified_block, new_entry_added = increment_version(
        latest_version, all_versions, gas_versions_block
    )
    # Step 5: Replace the old gas_feature_versions block with the new one
    content = update_gas_versions_block(content, modified_block)

    # Step 6: Update the LATEST_GAS_FEATURE_VERSION to the new version
    updated_content = update_latest_version(content, new_version)

    # Step 7: Write the updated content back to the file
    write_file(file_path, updated_content)

    if new_entry_added:
        print(f"Registered new gas feature version {new_version}")
    print(f"Updated LATEST_GAS_FEATURE_VERSION to {new_version}.")

if __name__ == "__main__":
    main()
