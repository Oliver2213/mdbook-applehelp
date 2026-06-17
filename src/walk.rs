use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use mdbook_renderer::book::{Book, BookItem, Chapter};

use crate::config::AppleHelpConfig;
use crate::html::{PageMeta, markdown_to_html_fragment, render_page};

pub struct WriteSummary {
    pub html_count: usize,
}

pub fn write_book(book: &Book, lproj: &Path, cfg: &AppleHelpConfig) -> Result<WriteSummary> {
    let landing_source: Option<PathBuf> = cfg.landing_page.as_ref().map(PathBuf::from);
    let mut landing_assigned = false;
    let mut html_count = 0usize;

    for item in &book.items {
        write_item(
            item,
            lproj,
            cfg,
            landing_source.as_deref(),
            &mut landing_assigned,
            &mut html_count,
        )?;
    }

    if !landing_assigned {
        log::warn!(
            "no chapter was rendered as the landing page (index.html) — \
             Help Viewer may not open the book correctly"
        );
    }

    Ok(WriteSummary { html_count })
}

fn write_item(
    item: &BookItem,
    lproj: &Path,
    cfg: &AppleHelpConfig,
    landing_source: Option<&Path>,
    landing_assigned: &mut bool,
    html_count: &mut usize,
) -> Result<()> {
    match item {
        BookItem::Chapter(ch) => {
            if let Some(path) = ch.path.as_ref() {
                let is_landing = match landing_source {
                    Some(p) => paths_equivalent(path, p),
                    None => !*landing_assigned,
                };
                write_chapter(ch, path, lproj, cfg, is_landing)?;
                if is_landing {
                    *landing_assigned = true;
                }
                *html_count += 1;
            }
            for sub in &ch.sub_items {
                write_item(sub, lproj, cfg, landing_source, landing_assigned, html_count)?;
            }
            Ok(())
        }
        BookItem::Separator | BookItem::PartTitle(_) => Ok(()),
    }
}

fn write_chapter(
    ch: &Chapter,
    source_path: &Path,
    lproj: &Path,
    cfg: &AppleHelpConfig,
    is_landing: bool,
) -> Result<()> {
    let rel_html: PathBuf = if is_landing {
        PathBuf::from("index.html")
    } else {
        with_html_extension(source_path)
    };
    let out_path = lproj.join(&rel_html);

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }

    let body = markdown_to_html_fragment(&ch.content);

    let anchor = anchor_for_path(source_path);
    let stylesheet = stylesheet_relative_from(&rel_html);

    let apple_title = if is_landing { &cfg.title } else { &ch.name };
    let page_title = if ch.name.is_empty() { &cfg.title } else { &ch.name };

    let meta = PageMeta {
        title: page_title,
        apple_title,
        language: &cfg.language,
        stylesheet_relative: &stylesheet,
        anchor: Some(&anchor),
    };

    let html = render_page(&body, &meta, cfg);
    fs::write(&out_path, html).with_context(|| format!("writing {}", out_path.display()))?;
    Ok(())
}

fn with_html_extension(p: &Path) -> PathBuf {
    let mut out = p.to_path_buf();
    out.set_extension("html");
    out
}

fn paths_equivalent(a: &Path, b: &Path) -> bool {
    a.with_extension("") == b.with_extension("")
}

/// Build a stable anchor name from a chapter source path.
/// `chapter1/sub-topic.md` → `chapter1/sub-topic`.
fn anchor_for_path(p: &Path) -> String {
    let stem: PathBuf = p.with_extension("");
    stem.to_string_lossy().replace('\\', "/")
}

/// Relative path from a chapter's HTML location back to `Shared/style.css`.
/// `index.html` → `../Shared/style.css`; `chapter-1/sub.html` → `../../Shared/style.css`.
fn stylesheet_relative_from(rel_html: &Path) -> String {
    let dirs_above = rel_html.components().count();
    let prefix = "../".repeat(dirs_above);
    format!("{prefix}Shared/style.css")
}
