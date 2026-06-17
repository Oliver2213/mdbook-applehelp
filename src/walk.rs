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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{IndexFormat, IndexOverride};
    use mdbook_renderer::book::{Book, Chapter};

    #[test]
    fn with_html_extension_replaces_md() {
        assert_eq!(
            with_html_extension(Path::new("foo.md")),
            PathBuf::from("foo.html")
        );
        assert_eq!(
            with_html_extension(Path::new("a/b/c.md")),
            PathBuf::from("a/b/c.html")
        );
    }

    /// Used to match a configured `landing-page` against a chapter's source
    /// path — extensions vary (`.md` vs `.html`) so we compare the stem.
    #[test]
    fn paths_equivalent_ignores_extension() {
        assert!(paths_equivalent(
            Path::new("intro.md"),
            Path::new("intro.html")
        ));
        assert!(paths_equivalent(Path::new("intro.md"), Path::new("intro")));
        assert!(paths_equivalent(
            Path::new("a/b.md"),
            Path::new("a/b.html")
        ));
        assert!(!paths_equivalent(
            Path::new("intro.md"),
            Path::new("outro.md")
        ));
        assert!(!paths_equivalent(
            Path::new("a/b.md"),
            Path::new("a/c.md")
        ));
    }

    /// Anchor names embedded in each page (`<a name="...">`) come from the
    /// chapter's source path, sans extension, normalized to forward slashes.
    #[test]
    fn anchor_for_path_strips_extension() {
        assert_eq!(anchor_for_path(Path::new("intro.md")), "intro");
        assert_eq!(
            anchor_for_path(Path::new("getting-started/sub.md")),
            "getting-started/sub"
        );
    }

    /// `<lproj>/index.html` → `../Shared/style.css` (one `..` to escape `<lproj>`).
    #[test]
    fn stylesheet_depth_root_is_one_up() {
        assert_eq!(
            stylesheet_relative_from(Path::new("index.html")),
            "../Shared/style.css"
        );
    }

    /// `<lproj>/getting-started/sub.html` → `../../Shared/style.css`.
    #[test]
    fn stylesheet_depth_one_nested_is_two_up() {
        assert_eq!(
            stylesheet_relative_from(Path::new("getting-started/sub.html")),
            "../../Shared/style.css"
        );
    }

    #[test]
    fn stylesheet_depth_two_nested_is_three_up() {
        assert_eq!(
            stylesheet_relative_from(Path::new("a/b/c.html")),
            "../../../Shared/style.css"
        );
    }

    fn cfg_for_book(title: &str) -> AppleHelpConfig {
        AppleHelpConfig {
            help_book_name: "com.test.help".into(),
            help_book_folder: "TestHelp".into(),
            title: title.into(),
            description: String::new(),
            language: "en".into(),
            authors: vec![],
            index_format: IndexFormat::Both,
            generate_index: false,
            landing_page: None,
            icon_file: None,
            external_url: None,
            access_key: None,
            index_override: IndexOverride::None,
        }
    }

    /// End-to-end: build a Book in memory with a nested sub-chapter, write it
    /// to a tempdir, and verify the on-disk layout matches the spec — landing
    /// page renamed to `index.html`, nested paths mirrored, AppleTitle and
    /// stylesheet depth correct per file, and `.md` links rewritten to `.html`.
    #[test]
    fn write_book_produces_expected_layout() {
        let intro = Chapter::new(
            "Welcome",
            "# Welcome\n\nGo to [parent](./parent.md).\n".to_string(),
            "intro.md",
            vec![],
        );
        let child = Chapter::new(
            "Child",
            "# Child\n\nBack to [intro](../intro.md).\n".to_string(),
            "parent/child.md",
            vec!["Parent".to_string()],
        );
        let mut parent = Chapter::new(
            "Parent",
            "# Parent\n".to_string(),
            "parent.md",
            vec![],
        );
        parent.sub_items = vec![BookItem::Chapter(child)];

        let book = Book::new_with_items(vec![
            BookItem::Chapter(intro),
            BookItem::Chapter(parent),
        ]);

        let tmp = tempfile::tempdir().unwrap();
        let lproj = tmp.path().join("en.lproj");
        std::fs::create_dir_all(&lproj).unwrap();

        let cfg = cfg_for_book("Welcome Guide");
        let summary = write_book(&book, &lproj, &cfg).unwrap();

        assert_eq!(summary.html_count, 3, "three chapters were rendered");

        let index = std::fs::read_to_string(lproj.join("index.html")).unwrap();
        let parent_html = std::fs::read_to_string(lproj.join("parent.html")).unwrap();
        let child_html =
            std::fs::read_to_string(lproj.join("parent").join("child.html")).unwrap();

        // First chapter (intro.md) became the landing page (index.html). The
        // file at `lproj/intro.html` should NOT exist, since intro got renamed.
        assert!(
            !lproj.join("intro.html").exists(),
            "landing chapter shouldn't also be at its original filename"
        );

        // AppleTitle on landing = book title; on other pages = chapter name.
        assert!(
            index.contains(r#"<meta name="AppleTitle" content="Welcome Guide">"#),
            "landing AppleTitle: {index}"
        );
        assert!(
            parent_html.contains(r#"<meta name="AppleTitle" content="Parent">"#),
            "parent AppleTitle: {parent_html}"
        );
        assert!(
            child_html.contains(r#"<meta name="AppleTitle" content="Child">"#),
            "child AppleTitle: {child_html}"
        );

        // Stylesheet path depth scales with nesting.
        assert!(
            index.contains(r#"href="../Shared/style.css""#),
            "landing stylesheet depth: {index}"
        );
        assert!(
            child_html.contains(r#"href="../../Shared/style.css""#),
            "child stylesheet depth: {child_html}"
        );

        // Anchors come from source paths.
        assert!(
            index.contains(r#"<a name="intro">"#),
            "landing anchor (from source path): {index}"
        );
        assert!(
            child_html.contains(r#"<a name="parent/child">"#),
            "child anchor: {child_html}"
        );

        // Markdown body rendered, .md links rewritten to .html.
        assert!(
            index.contains(r#"<a href="./parent.html">parent</a>"#),
            "rewritten sibling link: {index}"
        );
        assert!(
            child_html.contains(r#"<a href="../intro.html">intro</a>"#),
            "rewritten parent link: {child_html}"
        );
    }

    /// Draft chapters (no `path`) should be skipped entirely — no file written
    /// and no html_count increment.
    #[test]
    fn write_book_skips_draft_chapters() {
        let real = Chapter::new("Real", "# Real\n".to_string(), "real.md", vec![]);
        let draft = Chapter::new_draft("Draft", vec![]);
        let book = Book::new_with_items(vec![
            BookItem::Chapter(real),
            BookItem::Chapter(draft),
        ]);

        let tmp = tempfile::tempdir().unwrap();
        let lproj = tmp.path().join("en.lproj");
        std::fs::create_dir_all(&lproj).unwrap();
        let cfg = cfg_for_book("T");

        let summary = write_book(&book, &lproj, &cfg).unwrap();
        assert_eq!(summary.html_count, 1);
        // Only index.html (from the real chapter promoted to landing).
        assert!(lproj.join("index.html").exists());
    }

    /// Separator and PartTitle items shouldn't emit files or be counted.
    #[test]
    fn write_book_ignores_separators_and_part_titles() {
        let ch = Chapter::new("C", "# C\n".to_string(), "c.md", vec![]);
        let book = Book::new_with_items(vec![
            BookItem::PartTitle("Part One".into()),
            BookItem::Chapter(ch),
            BookItem::Separator,
        ]);
        let tmp = tempfile::tempdir().unwrap();
        let lproj = tmp.path().join("en.lproj");
        std::fs::create_dir_all(&lproj).unwrap();
        let cfg = cfg_for_book("T");

        let summary = write_book(&book, &lproj, &cfg).unwrap();
        assert_eq!(summary.html_count, 1);
        let entries: Vec<_> = std::fs::read_dir(&lproj).unwrap().collect();
        // Just index.html — no stray files from PartTitle or Separator.
        assert_eq!(entries.len(), 1);
    }

    /// `landing-page` configures which source chapter is renamed to
    /// `index.html`. When set, the named chapter wins over "first chapter".
    #[test]
    fn write_book_honors_explicit_landing_page() {
        let intro = Chapter::new("Intro", "# Intro\n".to_string(), "intro.md", vec![]);
        let cover = Chapter::new("Cover", "# Cover\n".to_string(), "cover.md", vec![]);
        let book = Book::new_with_items(vec![
            BookItem::Chapter(intro),
            BookItem::Chapter(cover),
        ]);
        let tmp = tempfile::tempdir().unwrap();
        let lproj = tmp.path().join("en.lproj");
        std::fs::create_dir_all(&lproj).unwrap();
        let mut cfg = cfg_for_book("T");
        cfg.landing_page = Some("cover.md".into());

        write_book(&book, &lproj, &cfg).unwrap();

        // `cover.md` became `index.html`; `intro.md` kept its own filename.
        assert!(lproj.join("index.html").exists());
        assert!(lproj.join("intro.html").exists());
        assert!(!lproj.join("cover.html").exists());
        // Sanity: the landing content is from cover, not intro.
        let index = std::fs::read_to_string(lproj.join("index.html")).unwrap();
        assert!(index.contains("<h1>Cover</h1>"), "landing body: {index}");
    }
}
