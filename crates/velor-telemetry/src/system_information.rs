// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::utils;
use velor_infallible::Mutex;
use velor_telemetry_service::types::telemetry::TelemetryEvent;
use once_cell::sync::Lazy;
use std::collections::BTreeMap;
use sysinfo::{CpuExt, DiskExt, System, SystemExt};

/// System information event name
const VELOR_NODE_SYSTEM_INFORMATION: &str = "VELOR_NODE_SYSTEM_INFORMATION";

/// System information keys
const CPU_BRAND: &str = "cpu_brand";
const CPU_COUNT: &str = "cpu_count";
const CPU_CORE_COUNT: &str = "cpu_core_count";
const CPU_FREQUENCY: &str = "cpu_frequency";
const CPU_NAME: &str = "cpu_name";
const CPU_VENDOR_ID: &str = "cpu_vendor_id";
const DISK_AVAILABLE_SPACE: &str = "disk_available_space";
const DISK_COUNT: &str = "disk_count";
const DISK_FILE_SYSTEM: &str = "disk_file_system";
const DISK_NAME: &str = "disk_name";
const DISK_TOTAL_SPACE: &str = "disk_total_space";
const DISK_TYPE: &str = "disk_type";
const MEMORY_AVAILABLE: &str = "memory_available";
const MEMORY_TOTAL: &str = "memory_total";
const MEMORY_USED: &str = "memory_used";
const SYSTEM_HOST_NAME: &str = "system_host_name";
const SYSTEM_KERNEL_VERSION: &str = "system_kernel_version";
const SYSTEM_NAME: &str = "system_name";
const SYSTEM_OS_VERSION: &str = "system_os_version";

/// Global system singleton (to avoid recreations)
pub static GLOBAL_SYSTEM: Lazy<Mutex<System>> = Lazy::new(|| Mutex::new(System::new_all()));

/// Collects and sends the build information via telemetry
pub(crate) async fn create_system_info_telemetry_event() -> TelemetryEvent {
    // Collect the system information
    let system_information = get_system_information();

    // Create and return a new telemetry event
    TelemetryEvent {
        name: VELOR_NODE_SYSTEM_INFORMATION.into(),
        params: system_information,
    }
}

/// Used to expose system information
pub fn get_system_information() -> BTreeMap<String, String> {
    let mut system_information: BTreeMap<String, String> = BTreeMap::new();
    collect_system_info(&mut system_information);
    system_information
}

/// Collects the system info and appends it to the given map
pub(crate) fn collect_system_info(system_information: &mut BTreeMap<String, String>) {
    // Note: this might be expensive, so it shouldn't be done often
    GLOBAL_SYSTEM.lock().refresh_system();
    GLOBAL_SYSTEM.lock().refresh_disks();

    // Collect relevant and available system information
    collect_cpu_info(system_information, &GLOBAL_SYSTEM);
    collect_disk_info(system_information, &GLOBAL_SYSTEM);
    collect_memory_info(system_information, &GLOBAL_SYSTEM);
    collect_sys_info(system_information, &GLOBAL_SYSTEM);
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
    utils::insert_optional_value(
        system_information,
        CPU_CORE_COUNT,
        system_lock
            .physical_core_count()
            .map(|count| count.to_string()),
    );

    // Collect the overall CPU info
    let global_cpu = system_lock.global_cpu_info();
    system_information.insert(CPU_BRAND.into(), global_cpu.brand().into());
    system_information.insert(CPU_FREQUENCY.into(), global_cpu.frequency().to_string());
    system_information.insert(CPU_NAME.into(), global_cpu.name().into());
    system_information.insert(CPU_VENDOR_ID.into(), global_cpu.vendor_id().into());
}

/// Collects the disk info and appends it to the given map
fn collect_disk_info(
    system_information: &mut BTreeMap<String, String>,
    system: &Lazy<Mutex<System>>,
) {
    // Collect the number of disks
    let system_lock = system.lock();
    let disks = system_lock.disks();
    utils::insert_optional_value(
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
    system_information.insert(
        DISK_AVAILABLE_SPACE.into(),
        disk.available_space().to_string(),
    );
    system_information.insert(DISK_FILE_SYSTEM.into(), format!("{:?}", disk.file_system()));
    system_information.insert(DISK_NAME.into(), format!("{:?}", disk.name()));
    system_information.insert(DISK_TOTAL_SPACE.into(), disk.total_space().to_string());
    system_information.insert(DISK_TYPE.into(), format!("{:?}", disk.type_()));
}

/// Collects the memory info and appends it to the given map
fn collect_memory_info(
    system_information: &mut BTreeMap<String, String>,
    system: &Lazy<Mutex<System>>,
) {
    // Collect the information for the memory
    let system_lock = system.lock();
    system_information.insert(
        MEMORY_AVAILABLE.into(),
        system_lock.available_memory().to_string(),
    );
    system_information.insert(MEMORY_TOTAL.into(), system_lock.total_memory().to_string());
    system_information.insert(MEMORY_USED.into(), system_lock.used_memory().to_string());
}

/// Collects the sys info and appends it to the given map
fn collect_sys_info(
    system_information: &mut BTreeMap<String, String>,
    system: &Lazy<Mutex<System>>,
) {
    utils::insert_optional_value(
        system_information,
        SYSTEM_HOST_NAME,
        system.lock().host_name(),
    );
    utils::insert_optional_value(
        system_information,
        SYSTEM_KERNEL_VERSION,
        system.lock().kernel_version(),
    );
    utils::insert_optional_value(system_information, SYSTEM_NAME, system.lock().name());
    utils::insert_optional_value(
        system_information,
        SYSTEM_OS_VERSION,
        system.lock().long_os_version(),
    );
}
