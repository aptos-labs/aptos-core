# Flags for performance builds. They must be kept in sync with the ones defined
# in `.cargo/config.toml`, with `-C linker-plugin-lto` added at the end.
PERFORMANCE_RUSTFLAGS=(
  "--cfg"
  "tokio_unstable"
  "-C"
  "link-arg=-fuse-ld=lld"
  "-C"
  "force-frame-pointers=yes"
  "-C"
  "force-unwind-tables=yes"
  "-C"
  "target-cpu=x86-64-v3"
  "-C"
  "linker-plugin-lto"
)
