use anyhow::{anyhow, Context, Result};
use clap::Parser;
use std::fs;
use std::path::PathBuf;

const HUNK_HEADER: u32 = 0x3f3;
const HUNK_CODE: u32 = 0x3e9;
const HUNK_DATA: u32 = 0x3ea;
const HUNK_BSS: u32 = 0x3eb;
const HUNK_RELOC32: u32 = 0x3ec;
const HUNK_SYMBOL: u32 = 0x3f0;
const HUNK_DEBUG: u32 = 0x3f1;
const HUNK_END: u32 = 0x3f2;
const HUNK_RELOC32SHORT: u32 = 0x3fc;
const HUNK_TYPE_MASK: u32 = 0x3fff_ffff;

#[derive(Parser)]
#[command(name = "amiga-version", version, about = "Print version strings from Amiga hunk files")]
struct Cli {
    /// Files to inspect.
    #[arg(required = true)]
    files: Vec<PathBuf>,
}

#[derive(Default)]
struct Hunk {
    data: Vec<u8>,
    relocations: Vec<(usize, usize)>,
}

fn read_u32(data: &[u8], pos: usize) -> Result<u32> {
    let bytes: [u8; 4] = data
        .get(pos..pos.saturating_add(4))
        .ok_or_else(|| anyhow!("truncated hunk file"))?
        .try_into()
        .map_err(|_| anyhow!("truncated hunk file"))?;
    Ok(u32::from_be_bytes(bytes))
}

fn read_u16(data: &[u8], pos: usize) -> Result<u16> {
    let bytes: [u8; 2] = data
        .get(pos..pos.saturating_add(2))
        .ok_or_else(|| anyhow!("truncated hunk file"))?
        .try_into()
        .map_err(|_| anyhow!("truncated hunk file"))?;
    Ok(u16::from_be_bytes(bytes))
}

fn advance(pos: &mut usize, amount: usize, len: usize) -> Result<()> {
    *pos = pos
        .checked_add(amount)
        .filter(|next| *next <= len)
        .ok_or_else(|| anyhow!("truncated hunk file"))?;
    Ok(())
}

fn parse_hunks(data: &[u8]) -> Result<Vec<Hunk>> {
    if read_u32(data, 0)? != HUNK_HEADER {
        return Err(anyhow!("not a hunk file"));
    }
    let mut pos = 4;
    while read_u32(data, pos)? != 0 {
        let words = read_u32(data, pos)? as usize;
        advance(&mut pos, 4usize.checked_add(words.checked_mul(4).ok_or_else(|| anyhow!("hunk size overflow"))?).ok_or_else(|| anyhow!("hunk size overflow"))?, data.len())?;
    }
    advance(&mut pos, 4, data.len())?;
    let count = read_u32(data, pos)? as usize;
    let first = read_u32(data, pos + 4)? as usize;
    let last = read_u32(data, pos + 8)? as usize;
    if last < first || last - first + 1 != count {
        return Err(anyhow!("invalid hunk table"));
    }
    let table_bytes = count.checked_mul(4).ok_or_else(|| anyhow!("hunk size overflow"))?;
    advance(&mut pos, 12usize.checked_add(table_bytes).ok_or_else(|| anyhow!("hunk size overflow"))?, data.len())?;

    let mut hunks: Vec<Hunk> = Vec::with_capacity(count);
    while pos < data.len() {
        let kind = read_u32(data, pos)? & HUNK_TYPE_MASK;
        advance(&mut pos, 4, data.len())?;
        match kind {
            HUNK_CODE | HUNK_DATA => {
                let words = (read_u32(data, pos)? & HUNK_TYPE_MASK) as usize;
                let bytes = words.checked_mul(4).ok_or_else(|| anyhow!("hunk size overflow"))?;
                let start = pos.checked_add(4).ok_or_else(|| anyhow!("hunk size overflow"))?;
                let end = start.checked_add(bytes).filter(|end| *end <= data.len()).ok_or_else(|| anyhow!("truncated hunk data"))?;
                hunks.push(Hunk { data: data[start..end].to_vec(), relocations: Vec::new() });
                pos = end;
            }
            HUNK_BSS => {
                hunks.push(Hunk::default());
                advance(&mut pos, 4, data.len())?;
            }
            HUNK_RELOC32 => {
                let current = hunks.last_mut().ok_or_else(|| anyhow!("relocations before first hunk"))?;
                loop {
                    let n = read_u32(data, pos)? as usize;
                    advance(&mut pos, 4, data.len())?;
                    if n == 0 { break; }
                    let target = read_u32(data, pos)? as usize;
                    advance(&mut pos, 4, data.len())?;
                    for _ in 0..n {
                        let offset = read_u32(data, pos)? as usize;
                        advance(&mut pos, 4, data.len())?;
                        current.relocations.push((offset, target));
                    }
                }
            }
            HUNK_RELOC32SHORT => {
                let current = hunks.last_mut().ok_or_else(|| anyhow!("relocations before first hunk"))?;
                let start = pos;
                loop {
                    let n = read_u16(data, pos)? as usize;
                    advance(&mut pos, 2, data.len())?;
                    if n == 0 { break; }
                    let target = read_u16(data, pos)? as usize;
                    advance(&mut pos, 2, data.len())?;
                    for _ in 0..n {
                        let offset = read_u16(data, pos)? as usize;
                        advance(&mut pos, 2, data.len())?;
                        current.relocations.push((offset, target));
                    }
                }
                if (pos - start) % 4 != 0 { advance(&mut pos, 2, data.len())?; }
            }
            HUNK_SYMBOL => loop {
                let words = read_u32(data, pos)? as usize;
                advance(&mut pos, 4, data.len())?;
                if words == 0 { break; }
                let bytes = words.checked_mul(4).and_then(|n| n.checked_add(4)).ok_or_else(|| anyhow!("hunk size overflow"))?;
                advance(&mut pos, bytes, data.len())?;
            },
            HUNK_DEBUG => {
                let words = read_u32(data, pos)? as usize;
                let bytes = words.checked_mul(4).and_then(|n| n.checked_add(4)).ok_or_else(|| anyhow!("hunk size overflow"))?;
                advance(&mut pos, bytes, data.len())?;
            }
            HUNK_END => {
                if hunks.len() == count { break; }
            }
            _ => return Err(anyhow!("unknown hunk type {kind:#x}")),
        }
    }
    if hunks.len() != count { return Err(anyhow!("hunk count does not match header")); }
    Ok(hunks)
}

fn ver_tag(data: &[u8]) -> Option<String> {
    let start = data.windows(6).position(|window| window == b"$VER: ")?;
    let text = &data[start + 6..];
    let end = text.iter().position(|byte| *byte == 0).unwrap_or(text.len());
    Some(String::from_utf8_lossy(&text[..end]).trim().to_string())
}

fn resident_tag(data: &[u8]) -> Result<Option<String>> {
    let hunks = parse_hunks(data)?;
    for (index, hunk) in hunks.iter().enumerate() {
        let mut search = 0;
        while let Some(relative) = hunk.data[search..].windows(2).position(|bytes| bytes == [0x4a, 0xfc]) {
            let pos = search + relative;
            if pos.checked_add(26).is_some_and(|end| end <= hunk.data.len())
                && read_u32(&hunk.data, pos + 2).ok() == Some(pos as u32)
                && hunk.relocations.contains(&(pos + 2, index))
            {
                let version = hunk.data[pos + 11];
                let id_ptr = read_u32(&hunk.data, pos + 18)? as usize;
                let target_index = hunk.relocations.iter().find(|(offset, _)| *offset == pos + 18).map(|(_, target)| *target).unwrap_or(index);
                let target = hunks.get(target_index).ok_or_else(|| anyhow!("resident ID hunk is out of range"))?;
                let ident = target.data.get(id_ptr..).ok_or_else(|| anyhow!("resident ID pointer is out of range"))?;
                let end = ident.iter().position(|byte| *byte == 0).unwrap_or(ident.len());
                let ident = String::from_utf8_lossy(&ident[..end]).trim().to_string();
                return Ok(Some(format!("{ident} (rt_Version {version})")));
            }
            search = pos + 2;
        }
    }
    Ok(None)
}

fn version(data: &[u8]) -> Result<Option<String>> {
    if let Some(version) = ver_tag(data) { return Ok(Some(version)); }
    resident_tag(data)
}

fn main() {
    let cli = Cli::parse();
    let mut status = 0;
    for path in cli.files {
        let result = fs::read(&path).with_context(|| format!("{}: unable to read file", path.display()))
            .and_then(|data| version(&data).with_context(|| format!("{}", path.display())));
        match result {
            Ok(Some(value)) => println!("{}: {value}", path.display()),
            Ok(None) => { eprintln!("{}: no version", path.display()); status = 1; }
            Err(error) => { eprintln!("{}: {error:#}", path.display()); status = 1; }
        }
    }
    std::process::exit(status);
}
