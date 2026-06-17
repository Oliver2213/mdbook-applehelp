use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use mdbook_renderer::RenderContext;

mod bundle;
mod config;
mod css;
mod html;
mod index;
mod walk;

use config::IndexOverride;

#[derive(Parser, Debug)]
#[command(
    name = "mdbook-applehelp",
    about = "An mdBook backend that generates a macOS Apple Help Book (.help bundle).",
    version
)]
struct Cli {
    /// Skip search-index generation regardless of book.toml.
    #[arg(long, conflicts_with = "force_index")]
    no_index: bool,

    /// Force search-index generation regardless of book.toml.
    #[arg(long)]
    force_index: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// mdBook backend support check. Exit 0 if `renderer` is "applehelp".
    Supports {
        /// Name of the renderer mdBook is asking about.
        renderer: String,
    },
}

fn main() -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .format_target(false)
        .init();

    let cli = Cli::parse();

    if let Some(Command::Supports { renderer }) = cli.command {
        return if renderer == "applehelp" {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }

    let index_override = match (cli.no_index, cli.force_index) {
        (true, _) => IndexOverride::ForceSkip,
        (_, true) => IndexOverride::ForceRun,
        _ => IndexOverride::None,
    };

    match run(index_override) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            log::error!("{err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(index_override: IndexOverride) -> Result<()> {
    let ctx =
        RenderContext::from_json(io::stdin().lock()).context("reading RenderContext from stdin")?;

    let cfg = config::AppleHelpConfig::from_context(&ctx, index_override)?;

    let bundle_root: PathBuf = ctx.destination.join(format!("{}.help", cfg.help_book_folder));
    let lproj = bundle_root
        .join("Contents")
        .join("Resources")
        .join(format!("{}.lproj", cfg.language));
    let shared = bundle_root.join("Contents").join("Resources").join("Shared");

    bundle::prepare_dirs(&bundle_root, &lproj, &shared)?;
    bundle::write_info_plist(&bundle_root, &cfg)?;
    css::write_default_css(&shared)?;

    let written = walk::write_book(&ctx.book, &lproj, &cfg)?;

    log::info!(
        "wrote {} HTML files to {}",
        written.html_count,
        lproj.display()
    );

    if cfg.should_generate_index() {
        index::generate(&lproj, &cfg)?;
    } else {
        log::info!("skipping search-index generation");
    }

    log::info!("Apple Help bundle: {}", bundle_root.display());
    Ok(())
}
