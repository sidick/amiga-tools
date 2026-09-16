# amiga-tools

Host-side command-line tools for classic Amiga development, in the spirit
of [amitools](https://github.com/cnvogelg/amitools) but built in Rust on
dedicated format crates.

| Tool | Does | Built on |
|------|------|----------|
| `amidisk` | ADF/HDF volume images: list, info, extract | [amiga-ffs](https://crates.io/crates/amiga-ffs) |
| `amirdb` | Rigid Disk Block partition tables: info, per-partition detail | [amiga-rdb](https://crates.io/crates/amiga-rdb) |
| `amirom` | Kickstart ROM images: info, resident scan, normalize, hi/lo split & merge | [amiga-rom](https://crates.io/crates/amiga-rom) |

All three are early skeletons; write-side commands (format, write, delete,
partition editing) are planned on the same crates.

## Building

```
cargo build --release
```

Binaries land in `target/release/{amidisk,amirdb,amirom}`.
