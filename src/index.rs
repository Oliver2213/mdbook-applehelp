use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};

use crate::config::{AppleHelpConfig, IndexFormat};

pub fn generate(lproj: &Path, cfg: &AppleHelpConfig) -> Result<()> {
    if !cfg!(target_os = "macos") {
        eprintln!("{}", non_macos_message(cfg));
        bail!("search-index generation requires macOS (hiutil unavailable)");
    }

    if Command::new("hiutil").arg("-h").output().is_err() {
        eprintln!("{}", non_macos_message(cfg));
        bail!("`hiutil` not found in PATH");
    }

    match cfg.index_format {
        IndexFormat::CoreSpotlight => run_hiutil(lproj, cfg, "corespotlight", &cfg.cshelp_index_filename())?,
        IndexFormat::Lsm => run_hiutil(lproj, cfg, "lsm", &cfg.lsm_index_filename())?,
        IndexFormat::Both => {
            run_hiutil(lproj, cfg, "corespotlight", &cfg.cshelp_index_filename())?;
            run_hiutil(lproj, cfg, "lsm", &cfg.lsm_index_filename())?;
        }
    }
    Ok(())
}

fn run_hiutil(lproj: &Path, cfg: &AppleHelpConfig, format: &str, output_file: &str) -> Result<()> {
    let output_path = lproj.join(output_file);
    log::info!(
        "hiutil -I {format} -Cf {} -a -s {lang} -l {lang} {}",
        output_path.display(),
        lproj.display(),
        lang = cfg.language
    );
    let status = Command::new("hiutil")
        .arg("-I")
        .arg(format)
        .arg("-Cf")
        .arg(&output_path)
        .arg("-a")
        .arg("-s")
        .arg(&cfg.language)
        .arg("-l")
        .arg(&cfg.language)
        .arg(lproj)
        .status()
        .with_context(|| format!("running hiutil for {format} index"))?;
    if !status.success() {
        return Err(anyhow!("hiutil ({format}) exited with status {status}"));
    }
    Ok(())
}

fn non_macos_message(cfg: &AppleHelpConfig) -> String {
    format!(
        "\u{26a0}  BUILD FAILED: Search index generation requires macOS (hiutil is unavailable).\n\n\
         \x20  Apple Help search indexes cannot be generated on this platform because\n\
         \x20  `hiutil` is a macOS-exclusive tool. The .help bundle was generated\n\
         \x20  without search indexes \u{2014} Help Viewer search will not work.\n\n\
         \x20  To resolve, do ONE of the following:\n\n\
         \x20  a) Disable index generation in book.toml:\n\
         \x20       [output.applehelp]\n\
         \x20       generate-index = false\n\n\
         \x20  b) Pass --no-index to skip for this build:\n\
         \x20       mdbook build  # then run on macOS:\n\
         \x20       mdbook-applehelp --force-index  # (outside mdbook, standalone)\n\n\
         \x20  c) Generate indexes on macOS after building:\n\
         \x20       cd <build-dir>/applehelp/<HelpBundle>.help/Contents/Resources/{lang}.lproj\n\
         \x20       hiutil -I corespotlight -Cf \"{cs}\" -a -s {lang} -l {lang} .\n\
         \x20       hiutil -I lsm -Cf \"{ls}\" -a -s {lang} -l {lang} .\n",
        lang = cfg.language,
        cs = cfg.cshelp_index_filename(),
        ls = cfg.lsm_index_filename(),
    )
}
