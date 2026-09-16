# amiga-tools

Host-side command-line tools for classic Amiga development, in the spirit
of [amitools](https://github.com/cnvogelg/amitools) but built in Rust on
dedicated format crates.

| Tool | Does | Built on |
|------|------|----------|
| `amidisk` | ADF/HDF volume images: create, format, list, read/write files, metadata (protect/comment/time/relabel), boot blocks, pack/unpack/repack, validate, repair, defrag, resize | [amiga-ffs](https://crates.io/crates/amiga-ffs) |
| `amirdb` | Rigid Disk Block partition tables: info, per-partition detail | [amiga-rdb](https://crates.io/crates/amiga-rdb) |
| `amirom` | Kickstart ROM images: info, resident scan, normalize, hi/lo split & merge | [amiga-rom](https://crates.io/crates/amiga-rom) |

`amidisk` covers the feature set of amitools' xdftool plus validation,
repair, defragmentation and resizing; `amirdb` and `amirom` are still
read-side skeletons.

## Building

```
cargo build --release
```

Binaries land in `target/release/{amidisk,amirdb,amirom}`.
