// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::system_metrics::GLOBAL_SYSTEM;
use aptos_infallible::Mutex;
use once_cell::sync::Lazy;
use shadow_rs::shadow;
use std::collections::BTreeMap;
use sysinfo::{CpuExt, DiskExt, System, SystemExt};

/// System information keys
const CPU: &str = "cpu";
const CPU_BRAND: &str = "cpu_brand";
const CPU_COUNT: &str = "cpu_count";
const CPU_CORE_COUNT: &str = "cpu_core_count";
const CPU_FREQUENCY: &str = "cpu_frequency";
const CPU_NAME: &str = "cpu_name";
const CPU_VENDOR_ID: &str = "cpu_vendor_id";
const DISK: &str = "disk";
const DISK_AVAILABLE_SPACE: &str = "disk_available_space";
const DISK_COUNT: &str = "disk_count";
const DISK_FILE_SYSTEM: &str = "disk_file_system";
const DISK_NAME: &str = "disk_name";
const DISK_TOTAL_SPACE: &str = "disk_total_space";
const DISK_TYPE: &str = "disk_type";
const SYSTEM: &str = "system";
const SYSTEM_HOST_NAME: &str = "system_host_name";
const SYSTEM_KERNEL_VERSION: &str = "system_kernel_version";
const SYSTEM_NAME: &str = "system_name";
const SYSTEM_OS_VERSION: &str = "system_os_version";

/// Build information keys
const BUILD: &str = "build";
const BUILD_BRANCH: &str = "build_branch";
const BUILD_CARGO_VERSION: &str = "build_cargo_version";
const BUILD_COMMIT_HASH: &str = "build_commit_hash";
const BUILD_OS: &str = "build_os";
const BUILD_PKG_VERSION: &str = "build_pkg_version";
const BUILD_PROJECT_NAME: &str = "build_project_name";
const BUILD_RUST_CHANNEL: &str = "build_rust_channel";
const BUILD_RUST_VERSION: &str = "build_rust_version";
const BUILD_TAG: &str = "build_tag";
const BUILD_TARGET: &str = "build_target";
const BUILD_TARGET_ARCH: &str = "build_target_arch";
const BUILD_TIME: &str = "build_time";
const BUILD_VERSION: &str = "build_version";

// Get access to BUILD information
shadow!(build);

/// Used to expose build information
pub fn get_build_information() -> BTreeMap<String, String> {
    let mut build_information: BTreeMap<String, String> = BTreeMap::new();
    collect_build_info(&mut build_information);
    build_information
}

/// Used to expose system information
pub fn get_system_information() -> BTreeMap<String, String> {
    let mut system_information: BTreeMap<String, String> = BTreeMap::new();
    collect_system_info(&mut system_information);
    system_information
}

// TODO(joshlind): find a way of removing this.
/// Fetches the git revision from the environment variables
pub fn get_git_rev() -> String {
    env!("GIT_REV").to_string()
}

/// Collects the build info and appends it to the given map
fn collect_build_info(system_information: &mut BTreeMap<String, String>) {
    // Get all the information about the build
    let mut build_information: BTreeMap<String, String> = BTreeMap::new();
    build_information.insert(BUILD_BRANCH.into(), build::BRANCH.into());
    build_information.insert(BUILD_CARGO_VERSION.into(), build::CARGO_VERSION.into());
    build_information.insert(BUILD_COMMIT_HASH.into(), build::COMMIT_HASH.into());
    build_information.insert(BUILD_OS.into(), build::BUILD_OS.into());
    build_information.insert(BUILD_PKG_VERSION.into(), build::PKG_VERSION.into());
    build_information.insert(BUILD_PROJECT_NAME.into(), build::PROJECT_NAME.into());
    build_information.insert(BUILD_RUST_CHANNEL.into(), build::RUST_CHANNEL.into());
    build_information.insert(BUILD_RUST_VERSION.into(), build::RUST_VERSION.into());
    build_information.insert(BUILD_TAG.into(), build::TAG.into());
    build_information.insert(BUILD_TARGET.into(), build::BUILD_TARGET.into());
    build_information.insert(BUILD_TARGET_ARCH.into(), build::BUILD_TARGET_ARCH.into());
    build_information.insert(BUILD_TIME.into(), build::BUILD_TIME.into());
    build_information.insert(BUILD_VERSION.into(), build::VERSION.into());

    // Generate an entry for the build in the system information
    let build_information = serde_json::to_string(&build_information).unwrap();
    system_information.insert(BUILD.into(), build_information);
}

/// Collects the system info and appends it to the given map
fn collect_system_info(system_information: &mut BTreeMap<String, String>) {
    // Note: this might be expensive, so it shouldn't be done often.
    GLOBAL_SYSTEM.lock().refresh_system();
    GLOBAL_SYSTEM.lock().refresh_disks();

    // Collect relevant and available system information
    collect_cpu_info(system_information, &GLOBAL_SYSTEM);
    collect_disk_info(system_information, &GLOBAL_SYSTEM);
    collect_sys_info(system_information, &GLOBAL_SYSTEM);
}

/// Inserts an optional value into the given map iff the value exists
fn insert_optional_value(
    system_information: &mut BTreeMap<String, String>,
    key: &str,
    value: Option<String>,
) {
    if let Some(value) = value {
        system_information.insert(key.to_string(), value);
    }
}

/// Collects the sys info and appends it to the given map
fn collect_sys_info(
    system_information: &mut BTreeMap<String, String>,
    system: &Lazy<Mutex<System>>,
) {
    // Create a map for all sys info
    let mut sys_info: BTreeMap<String, String> = BTreeMap::new();
    insert_optional_value(&mut sys_info, SYSTEM_HOST_NAME, system.lock().host_name());
    insert_optional_value(
        &mut sys_info,
        SYSTEM_KERNEL_VERSION,
        system.lock().kernel_version(),
    );
    insert_optional_value(&mut sys_info, SYSTEM_NAME, system.lock().name());
    insert_optional_value(
        &mut sys_info,
        SYSTEM_OS_VERSION,
        system.lock().long_os_version(),
    );

    // Generate an entry for the sys info in the overall information
    let sys_info = serde_json::to_string(&sys_info).unwrap();
    system_information.insert(SYSTEM.into(), sys_info);
}

/// Collects the cpu info and appends it to the given map
fn collect_cpu_info(
    system_information: &mut BTreeMap<String, String>,
    system: &Lazy<Mutex<System>>,
) {
    // Collect the number of CPUs and cores
    let system_lock = system.lock();
    let cpus = system_lock.cpus();
    system_information.insert(CPU_COUNT.into(), cpus.len().to_string());
    insert_optional_value(
        system_information,
        CPU_CORE_COUNT,
        system_lock
            .physical_core_count()
            .map(|count| count.to_string()),
    );

    // Collect the overall CPU info
    let global_cpu = system_lock.global_cpu_info();
    let mut cpu_information: BTreeMap<String, String> = BTreeMap::new();
    cpu_information.insert(CPU_BRAND.into(), global_cpu.brand().into());
    cpu_information.insert(CPU_FREQUENCY.into(), global_cpu.frequency().to_string());
    cpu_information.insert(CPU_NAME.into(), global_cpu.name().into());
    cpu_information.insert(CPU_VENDOR_ID.into(), global_cpu.vendor_id().into());

    // Generate an entry for the CPU in the system information
    let cpu_information = serde_json::to_string(&cpu_information).unwrap();
    system_information.insert(CPU.into(), cpu_information);
}

/// Collects the disk info and appends it to the given map
fn collect_disk_info(
    system_information: &mut BTreeMap<String, String>,
    system: &Lazy<Mutex<System>>,
) {
    // Collect the number of disks
    let system_lock = system.lock();
    let disks = system_lock.disks();
    insert_optional_value(
        system_information,
        DISK_COUNT,
        Some(disks.len().to_string()),
    );

    // If there's no disks found, return.
    if disks.is_empty() {
        return;
    }

    // Identify the index of the largest disk
    let mut largest_disk_index = 0;
    let mut largest_disk_size = 0;
    for (index, disk) in disks.iter().enumerate() {
        let disk_size = disk.total_space();
        if disk_size > largest_disk_size {
            largest_disk_index = index;
            largest_disk_size = disk_size;
        }
    }

    // Collect the information for the largest disk
    let disk = &disks[largest_disk_index];
    let mut disk_information: BTreeMap<String, String> = BTreeMap::new();
    disk_information.insert(
        DISK_AVAILABLE_SPACE.into(),
        disk.available_space().to_string(),
    );
    disk_information.insert(DISK_FILE_SYSTEM.into(), format!("{:?}", disk.file_system()));
    disk_information.insert(DISK_NAME.into(), format!("{:?}", disk.name()));
    disk_information.insert(DISK_TOTAL_SPACE.into(), disk.total_space().to_string());
    disk_information.insert(DISK_TYPE.into(), format!("{:?}", disk.type_()));

    // Generate an entry for the disk in the system information
    let disk_information = serde_json::to_string(&disk_information).unwrap();
    system_information.insert(DISK.into(), disk_information);
}
