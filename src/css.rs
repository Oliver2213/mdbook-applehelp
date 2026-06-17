use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

const DEFAULT_CSS: &str = include_str!("../assets/style.css");

pub fn write_default_css(shared_dir: &Path) -> Result<()> {
    let path = shared_dir.join("style.css");
    fs::write(&path, DEFAULT_CSS).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}
