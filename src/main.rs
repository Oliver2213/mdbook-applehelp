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

const LONG_ABOUT: &str = "\
An mdBook backend that generates a macOS Apple Help Book (.help bundle), ready for imbedding in a .app.

This tool is invoked by `mdbook` itself — you don't normally run it directly. \
To enable it, add an [output.applehelp] table to your book.toml, then build \
the book with `mdbook build`.";

const AFTER_LONG_HELP: &str = "\
GETTING STARTED:
  1. Add an [output.applehelp] table to your book.toml:

       [output.applehelp]
       help-book-name   = \"com.example.myapp.help\"   # CFBundleHelpBookName
       help-book-folder = \"MyAppHelp\"                # bundle directory

  2. Build the book:

       mdbook build

  3. The bundle lands at <build-dir>/applehelp/MyAppHelp.help — copy it
     into your app's Contents/Resources/ directory.

OPTIONAL CONFIG (with defaults):
       generate-index = true            # run hiutil on macOS (false to skip)
       index-format   = \"both\"          # \"corespotlight\" | \"lsm\" | \"both\"
       landing-page   = \"intro.md\"      # source chapter to use as index.html
       icon-file      = \"Shared/icon.png\"
       external-url   = \"https://...\"   # for remote index updates

CLI FLAGS (override book.toml for one build):
  --no-index      Skip hiutil regardless of generate-index.
  --force-index   Run hiutil regardless of generate-index.

Pass these via the `command` key in book.toml:

       [output.applehelp]
       command = \"mdbook-applehelp --no-index\"

See https://codeberg.org/coffee_nebula/mdbook-applehelp for full docs.";

#[derive(Parser, Debug)]
#[command(
    name = "mdbook-applehelp",
    about = "An mdBook backend that generates a macOS Apple Help Book, ready to be imbedded in a .app.",
    long_about = LONG_ABOUT,
    after_long_help = AFTER_LONG_HELP,
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
