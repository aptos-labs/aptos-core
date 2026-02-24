// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at
// https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Remap the `.text` section of the current process onto 2MB huge pages.
//!
//! This reduces iTLB misses for large binaries like `aptos-node`. The approach
//! is inspired by HHVM's `hugifyText`: for each 2MB-aligned chunk in the
//! executable text region, allocate a 2MB huge page, copy the code, and
//! atomically replace the original mapping via `mremap(MREMAP_FIXED)`.
//!
//! Must be called early in `main()` before other threads are spawned.

/// Attempt to place the process's `.text` section on 2MB huge pages.
///
/// Returns the number of 2MB pages successfully remapped, or an error message.
/// On non-Linux platforms this is a no-op.
pub fn hugify_process_text() -> Result<usize, String> {
    #[cfg(target_os = "linux")]
    {
        linux::hugify_text()
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(0)
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::{
        fs,
        io::{self, BufRead},
    };

    const SIZE_2M: usize = 2 * 1024 * 1024;

    pub fn hugify_text() -> Result<usize, String> {
        let exe_path =
            fs::read_link("/proc/self/exe").map_err(|e| format!("read /proc/self/exe: {e}"))?;
        let exe_path_str = exe_path.to_string_lossy();

        let regions = find_text_regions(&exe_path_str)?;
        if regions.is_empty() {
            return Err("no r-xp regions found for the current binary".into());
        }

        // Determine which 2MB chunk contains our own code so we can skip it.
        // remap_to_huge_page is the function that calls mremap — if its own page
        // is being replaced while it executes, the kernel atomically swaps in an
        // identical copy so it *should* be fine, but HHVM avoids this out of
        // caution and so do we.
        let self_addr = remap_to_huge_page as usize;
        let self_page = round_down_2m(self_addr);

        let mut total_remapped = 0;
        for (start, end) in &regions {
            let aligned_start = round_up_2m(*start);
            let aligned_end = round_down_2m(*end);
            if aligned_start >= aligned_end {
                continue;
            }
            let size = aligned_end - aligned_start;
            let num_pages = size / SIZE_2M;

            eprintln!(
                "hugify: region {:#x}-{:#x} -> {num_pages} x 2MB pages ({:#x}-{:#x})",
                start, end, aligned_start, aligned_end,
            );

            for i in 0..num_pages {
                let addr = aligned_start + i * SIZE_2M;

                // Skip the 2MB chunk that contains our own remapping code.
                if addr == self_page {
                    eprintln!("hugify: skipping page at {addr:#x} (contains hugify code)");
                    continue;
                }

                match remap_to_huge_page(addr) {
                    Ok(()) => total_remapped += 1,
                    Err(e) => {
                        eprintln!("hugify: failed to remap page at {addr:#x}: {e}");
                        // Continue with remaining pages rather than aborting entirely.
                    },
                }
            }
        }

        Ok(total_remapped)
    }

    /// Parse `/proc/self/maps` to find all `r-xp` regions belonging to our binary.
    fn find_text_regions(exe_path: &str) -> Result<Vec<(usize, usize)>, String> {
        let file =
            fs::File::open("/proc/self/maps").map_err(|e| format!("open /proc/self/maps: {e}"))?;
        let reader = io::BufReader::new(file);

        let mut regions = Vec::new();
        for line in reader.lines() {
            let line = line.map_err(|e| format!("read /proc/self/maps: {e}"))?;
            // Format: 00400000-00452000 r-xp 00000000 08:02 173521 /path/to/binary
            let mut parts = line.split_whitespace();
            let addr_range = match parts.next() {
                Some(s) => s,
                None => continue,
            };
            let perms = match parts.next() {
                Some(s) => s,
                None => continue,
            };
            // Skip offset, dev, inode
            let _offset = parts.next();
            let _dev = parts.next();
            let _inode = parts.next();
            let pathname = match parts.next() {
                Some(s) => s,
                None => continue,
            };

            if perms != "r-xp" || pathname != exe_path {
                continue;
            }

            let (start_str, end_str) = addr_range
                .split_once('-')
                .ok_or_else(|| format!("bad address range: {addr_range}"))?;
            let start = usize::from_str_radix(start_str, 16)
                .map_err(|e| format!("parse addr {start_str}: {e}"))?;
            let end = usize::from_str_radix(end_str, 16)
                .map_err(|e| format!("parse addr {end_str}: {e}"))?;

            regions.push((start, end));
        }

        Ok(regions)
    }

    /// Remap one 2MB chunk at `addr` onto a hugetlb page.
    ///
    /// # Safety note
    ///
    /// The caller must ensure this function's own code is NOT within the 2MB
    /// chunk at `addr` (see `self_page` skip logic in `hugify_text`).
    fn remap_to_huge_page(addr: usize) -> Result<(), String> {
        unsafe {
            // Step 1: Allocate a new 2MB hugetlb page.
            let new_page = alloc_huge_page()?;

            // Step 2: Copy the code.
            std::ptr::copy_nonoverlapping(addr as *const u8, new_page as *mut u8, SIZE_2M);

            // Step 3: Set permissions to read+execute.
            if libc::mprotect(new_page, SIZE_2M, libc::PROT_READ | libc::PROT_EXEC) != 0 {
                let err = io::Error::last_os_error();
                libc::munmap(new_page, SIZE_2M);
                return Err(format!("mprotect: {err}"));
            }

            // Step 4: Atomically replace the original mapping.
            let result = libc::mremap(
                new_page,
                SIZE_2M,
                SIZE_2M,
                libc::MREMAP_MAYMOVE | libc::MREMAP_FIXED,
                addr as *mut libc::c_void,
            );

            if result == libc::MAP_FAILED {
                let err = io::Error::last_os_error();
                libc::munmap(new_page, SIZE_2M);
                return Err(format!("mremap: {err}"));
            }

            Ok(())
        }
    }

    /// Allocate a 2MB hugetlb page.
    ///
    /// Panics if hugetlb pages are not available — make sure to pre-allocate
    /// them via `/proc/sys/vm/nr_hugepages` before running.
    unsafe fn alloc_huge_page() -> Result<*mut libc::c_void, String> {
        let page = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                SIZE_2M,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_HUGETLB,
                -1,
                0,
            )
        };
        assert!(
            page != libc::MAP_FAILED,
            "MAP_HUGETLB mmap failed: {}. \
             Make sure to pre-allocate huge pages: \
             echo <N> | sudo tee /proc/sys/vm/nr_hugepages",
            io::Error::last_os_error(),
        );
        Ok(page)
    }

    fn round_up_2m(addr: usize) -> usize {
        (addr + SIZE_2M - 1) & !(SIZE_2M - 1)
    }

    fn round_down_2m(addr: usize) -> usize {
        addr & !(SIZE_2M - 1)
    }
}
