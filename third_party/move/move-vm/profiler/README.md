# VM Profiler

This is a lightweight Rust profiling utility for instrumenting function and instruction execution inside the Move VM.
It provides per-function and per-instruction timing via RAII guards
and the [usdt](https://docs.rs/usdt) crate.

---

## Usage

To use this profiler, build a binary with the `probe-profiler` feature enabled, and attach an external tracing tool
(e.g. `dtrace` on macOS or `bpftrace` on Linux).

Example using **DTrace**:

```bash
sudo dtrace -s <DTRACE_SCRIPT> -c <COMPILED_BINARY>
```

To generate a flamegraph, use the provided script with an appropriate binary:
```bash
./profile.sh <COMPILED_BINARY>
```
Note that this requires that you have `flamegraph.pl` installed locally, and depends on the `fold.awk` and `trace.d` scripts, which must be kept in the same directory.