//! `amidisk <image> bitmap [ami-path]` — allocation bitmap views.
//!
//! Args deliberately minimal for now; a later worker designs the
//! overall-map vs. per-file/dir presentation.

use std::path::Path;

use anyhow::Result;
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Restrict the view to one file or directory's own blocks.
    pub ami_path: Option<String>,
}

pub fn run(_image: &Path, _args: Args) -> Result<()> {
    anyhow::bail!("not yet implemented")
}
