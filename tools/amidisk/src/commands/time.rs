//! `amidisk <image> time <ami-path> <timestamp>` — change an entry's
//! modification timestamp.
//!
//! `timestamp` is `YYYY-MM-DD[ HH:MM[:SS[.ticks]]]` — an ISO-8601-ish
//! date, hand-parsed rather than pulled in as a dependency — or the
//! literal `now`, taken from the host clock.

use std::path::Path;
use std::time::SystemTime;

use amiga_ffs::populate::datestamp_from_system_time;
use amiga_ffs::{CalendarDate, MetaUpdate};
use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Path inside the volume.
    pub ami_path: String,

    /// New timestamp: `YYYY-MM-DD[ HH:MM[:SS[.ticks]]]`, or `now`.
    pub timestamp: String,
}

/// Parse one `YYYY-MM-DD[ HH:MM[:SS[.ticks]]]` timestamp. `ticks` are
/// 1/50 s, the unit `DateStamp` itself counts in, not milliseconds.
pub fn parse_timestamp(spec: &str) -> Result<CalendarDate> {
    let spec = spec.trim();
    let (date, time) = match spec.split_once([' ', 'T']) {
        Some((d, t)) => (d, Some(t)),
        None => (spec, None),
    };

    let mut parts = date.split('-');
    let (year, month, day) = (|| {
        let y = parts.next()?;
        let m = parts.next()?;
        let d = parts.next()?;
        if parts.next().is_some() {
            return None;
        }
        Some((y, m, d))
    })()
    .with_context(|| format!("{spec:?} is not YYYY-MM-DD[ HH:MM[:SS[.ticks]]]"))?;

    let year: i32 = year.parse().with_context(|| format!("{year:?} is not a year"))?;
    let month: u32 = month.parse().with_context(|| format!("{month:?} is not a month"))?;
    let day: u32 = day.parse().with_context(|| format!("{day:?} is not a day"))?;

    let (hour, minute, second, tick) = match time {
        None => (0, 0, 0, 0),
        Some(t) => {
            let mut hm = t.split(':');
            let h = hm.next().with_context(|| format!("{t:?} is not HH:MM[:SS[.ticks]]"))?;
            let m = hm.next().with_context(|| format!("{t:?} is not HH:MM[:SS[.ticks]]"))?;
            let rest = hm.next();
            if hm.next().is_some() {
                bail!("{t:?} is not HH:MM[:SS[.ticks]]");
            }
            let hour: u32 = h.parse().with_context(|| format!("{h:?} is not an hour"))?;
            let minute: u32 = m.parse().with_context(|| format!("{m:?} is not a minute"))?;
            let (second, tick) = match rest {
                None => (0, 0),
                Some(s) => match s.split_once('.') {
                    Some((sec, tick)) => {
                        let sec: u32 = sec.parse().with_context(|| format!("{sec:?} is not a second"))?;
                        let tick: u32 =
                            tick.parse().with_context(|| format!("{tick:?} is not a tick count"))?;
                        (sec, tick)
                    }
                    None => (s.parse().with_context(|| format!("{s:?} is not a second"))?, 0),
                },
            };
            (hour, minute, second, tick)
        }
    };

    Ok(CalendarDate {
        year,
        month,
        day,
        hour,
        minute,
        second,
        tick,
    })
}

pub fn run(image: &Path, args: Args) -> Result<()> {
    let calendar = if args.timestamp.trim().eq_ignore_ascii_case("now") {
        datestamp_from_system_time(SystemTime::now()).to_calendar()
    } else {
        parse_timestamp(&args.timestamp)?
    };
    let date = amiga_ffs::DateStamp::from_calendar(calendar)
        .with_context(|| format!("{calendar:?} is outside what a DateStamp can record (1978-01-01 onward)"))?;

    let mut mutator = super::open_mutator(image)?;
    let mut vol = mutator.volume();
    let root = vol.root_lba();
    let entry = vol
        .lookup_path(root, args.ami_path.as_bytes())
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("looking up {}", args.ami_path))?
        .with_context(|| format!("{}: not found", args.ami_path))?;

    mutator
        .set_metadata(entry.lba, &MetaUpdate::new().date(date))
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("setting timestamp on {}", args.ami_path))?;

    super::save_back(mutator, image)?;

    println!("{}: {}", args.ami_path, super::root::format_iso(date.to_calendar()));
    Ok(())
}
