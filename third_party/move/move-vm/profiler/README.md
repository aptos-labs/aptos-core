# VM Profiler

This is a lightweight Rust profiling utility for instrumenting function and instruction execution inside the Move VM.
It provides per-function and per-instruction timing via RAII guards
and the [usdt](https://docs.rs/usdt) crate.

---

## Usage

To use this profiler, build the binary with the `probe-profiler` feature enabled, and attach an external tracing tool
(e.g. `dtrace` on macOS or `bpftrace` on Linux).

Example using **DTrace**:

```bash
sudo dtrace -s <DTRACE_SCRIPT> -c <COMPILED_BINARY>
```
