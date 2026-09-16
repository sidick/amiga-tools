# amiga-tools

Host-side command-line tools for classic Amiga development, in the spirit
of [amitools](https://github.com/cnvogelg/amitools) but built in Rust on
dedicated format crates.

| Tool | Does | Built on |
|------|------|----------|
| `amidisk` | ADF/HDF volume images: create, format, list, read/write files, metadata (protect/comment/time/relabel), boot blocks, pack/unpack/repack, validate, repair, defrag, resize | [amiga-ffs](https://crates.io/crates/amiga-ffs) |
| `amirdb` | Rigid Disk Block partition tables: create/init, partition add/change/delete/fill, export/import, filesystem drivers, block map, geometry adjust/remap, validate | [amiga-rdb](https://crates.io/crates/amiga-rdb) |
| `amirom` | Kickstart ROM images: info, resident scan, normalize, hi/lo split & merge | [amiga-rom](https://crates.io/crates/amiga-rom) |

`amidisk` and `amirdb` cover the feature sets of amitools' xdftool and
rdbtool plus extras (validation, repair, defrag, resize, links, block
maps); `amirom` is still a read-side skeleton.

## Building

```
cargo build --release
```

Binaries land in `target/release/{amidisk,amirdb,amirom}`.
