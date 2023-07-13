// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0


pub struct ThreadProfiler {
    config: ThreadProfilerConfig,
}

impl ThreadProfiler {
    pub fn new(config: ThreadProfilerConfig) -> Self {
        Self {
            config
        }
    }
}

impl Profiler for ThreadProfiler {
    fn start_profiling(&self) {
        let trace = rstack_self::trace(
            Command::new("cargo")
                .arg("run")
                .arg("-p")
                .arg("aptos-iuchild")
                .arg("--release"),
        )
            .unwrap();

        // Open a file for writing
        let mut file = File::create("./thread_dump.txt").unwrap();

        // Write the trace information to the file
        write!(file, "{:#?}", trace).unwrap();
    }

    fn end_profiling(&self) {
        unimplemented!()
    }

    fn expose_results(&self) -> String {
        unimplemented!()
    }
}
