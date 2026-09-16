//! `amirom diff` — compare two ROM images and report differing byte
//! ranges. `romtool diff`'s rough equivalent: that tool prints every
//! differing row as a two-column hex dump; this instead coalesces
//! consecutive differing bytes into ranges (offset + length) and shows
//! each side's leading bytes, which reads better for the common case of
//! a handful of scattered edits rather than a wholesale rewrite.
//!
//! `--modules`-aware diffing (per-module comparison rather than raw
//! byte ranges) is deliberately out of scope for this pass — noted in
//! the task brief as "consider later".

use std::path::PathBuf;

use anyhow::Result;
use clap::Args as ClapArgs;

use crate::commands::{load_rom, KeyArg};

#[derive(ClapArgs)]
pub struct Args {
    /// First ROM image.
    pub a: PathBuf,
    /// Second ROM image.
    pub b: PathBuf,

    #[command(flatten)]
    pub key: KeyArg,

    /// Maximum number of differing ranges to print (0 = unlimited).
    #[arg(long, default_value_t = 20)]
    pub limit: usize,

    /// Bytes of each side to show per differing range.
    #[arg(long, default_value_t = 8)]
    pub context: usize,
}

pub fn run(args: Args) -> Result<()> {
    let key = args.key.load()?;
    let a = load_rom(&args.a, key.as_deref())?;
    let b = load_rom(&args.b, key.as_deref())?;

    if a.len() != b.len() {
        println!(
            "size mismatch: {} is {} bytes, {} is {} bytes",
            args.a.display(),
            a.len(),
            args.b.display(),
            b.len()
        );
    }

    let ranges = diff_ranges(&a, &b);
    let total = ranges.len();
    let shown = if args.limit == 0 { total } else { total.min(args.limit) };

    for (offset, len) in ranges.iter().take(shown) {
        let end = offset + len;
        print!("{offset:08x}  +{len:<6}  a: {}", hex_prefix(&a[*offset..end], args.context));
        print!("  |  b: {}", hex_prefix(&b[*offset..end], args.context));
        println!();
    }
    if shown < total {
        println!("... {} more differing range(s) not shown (--limit {})", total - shown, args.limit);
    }

    let diff_bytes: usize = ranges.iter().map(|(_, len)| len).sum();
    println!(
        "{total} differing range(s), {diff_bytes} byte(s) differ out of {} compared",
        a.len().min(b.len())
    );

    Ok(())
}

/// Coalesces every index where `a[i] != b[i]` (over the common prefix,
/// if lengths differ) into `(offset, length)` runs of consecutive
/// differing bytes.
fn diff_ranges(a: &[u8], b: &[u8]) -> Vec<(usize, usize)> {
    let common = a.len().min(b.len());
    let mut ranges = Vec::new();
    let mut run_start: Option<usize> = None;

    for i in 0..common {
        if a[i] != b[i] {
            if run_start.is_none() {
                run_start = Some(i);
            }
        } else if let Some(start) = run_start.take() {
            ranges.push((start, i - start));
        }
    }
    if let Some(start) = run_start {
        ranges.push((start, common - start));
    }
    ranges
}

/// First `n` bytes of `data` as a hex string, `...` appended if `data`
/// is longer.
fn hex_prefix(data: &[u8], n: usize) -> String {
    let shown = data.len().min(n);
    let mut s = data[..shown].iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ");
    if data.len() > shown {
        s.push_str(" ...");
    }
    s
}
