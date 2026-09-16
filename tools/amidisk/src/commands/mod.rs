//! One module per subcommand, so `format`/`write`/`delete` can be added
//! later without disturbing the ones that already exist.

pub mod info;
pub mod list;
pub mod read;

/// Amiga filenames are Latin-1-ish bytes, not UTF-8. This is a display
/// approximation only: bytes 0x80-0xFF map to the matching Unicode code
/// points, which happens to be correct for Latin-1 but not for whatever
/// codepage a given disk's author actually typed in.
pub fn display_name(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}
