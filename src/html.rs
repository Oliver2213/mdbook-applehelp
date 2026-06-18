use pulldown_cmark::{CowStr, Event, LinkType, Options, Parser, Tag, html};

use crate::config::AppleHelpConfig;

/// Convert Markdown to an HTML fragment, rewriting internal `.md` links to `.html`.
pub fn markdown_to_html_fragment(markdown: &str) -> String {
    let opts = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_SMART_PUNCTUATION
        | Options::ENABLE_HEADING_ATTRIBUTES;

    let parser = Parser::new_ext(markdown, opts).map(|event| match event {
        Event::Start(Tag::Link {
            link_type,
            dest_url,
            title,
            id,
        }) => Event::Start(Tag::Link {
            link_type,
            dest_url: rewrite_link(&dest_url, link_type),
            title,
            id,
        }),
        Event::Start(Tag::Image {
            link_type,
            dest_url,
            title,
            id,
        }) => Event::Start(Tag::Image {
            link_type,
            dest_url: rewrite_link(&dest_url, link_type),
            title,
            id,
        }),
        other => other,
    });

    let mut out = String::new();
    html::push_html(&mut out, parser);
    out
}

fn rewrite_link<'a>(url: &CowStr<'a>, link_type: LinkType) -> CowStr<'a> {
    if matches!(link_type, LinkType::Email | LinkType::Autolink) {
        return CowStr::Boxed(url.to_string().into_boxed_str());
    }
    if is_external(url) {
        return CowStr::Boxed(url.to_string().into_boxed_str());
    }
    let (path, fragment) = split_fragment(url);
    let new_path = if let Some(stem) = path.strip_suffix(".md") {
        format!("{stem}.html")
    } else if let Some(stem) = path.strip_suffix(".markdown") {
        format!("{stem}.html")
    } else {
        return CowStr::Boxed(url.to_string().into_boxed_str());
    };
    let combined = match fragment {
        Some(f) => format!("{new_path}#{f}"),
        None => new_path,
    };
    CowStr::Boxed(combined.into_boxed_str())
}

fn is_external(url: &str) -> bool {
    if url.starts_with('#') || url.starts_with('/') {
        return true;
    }
    if let Some(idx) = url.find(':') {
        let scheme = &url[..idx];
        if !scheme.is_empty() && scheme.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.') {
            return true;
        }
    }
    false
}

fn split_fragment(url: &str) -> (&str, Option<&str>) {
    match url.find('#') {
        Some(idx) => (&url[..idx], Some(&url[idx + 1..])),
        None => (url, None),
    }
}

/// Wrap a body fragment in an Apple Help-compatible HTML document.
pub struct PageMeta<'a> {
    pub title: &'a str,
    pub apple_title: &'a str,
    pub language: &'a str,
    pub stylesheet_relative: &'a str,
    pub anchor: Option<&'a str>,
}

pub fn render_page(body_fragment: &str, meta: &PageMeta<'_>, cfg: &AppleHelpConfig) -> String {
    let mut head = String::new();
    head.push_str("<!DOCTYPE html>\n");
    head.push_str(&format!(
        "<html lang=\"{}\">\n<head>\n",
        escape(meta.language)
    ));
    head.push_str("    <meta http-equiv=\"Content-Type\" content=\"text/html; charset=utf-8\">\n");
    head.push_str(&format!(
        "    <meta name=\"AppleTitle\" content=\"{}\">\n",
        escape(meta.apple_title)
    ));
    if let Some(icon) = &cfg.icon_file {
        head.push_str(&format!(
            "    <meta name=\"AppleIcon\" content=\"{}\">\n",
            escape(icon)
        ));
    }
    if !cfg.description.is_empty() {
        head.push_str(&format!(
            "    <meta name=\"description\" content=\"{}\">\n",
            escape(&cfg.description)
        ));
    }
    if !cfg.authors.is_empty() {
        head.push_str(&format!(
            "    <meta name=\"author\" content=\"{}\">\n",
            escape(&cfg.authors.join(", "))
        ));
    }
    head.push_str(&format!("    <title>{}</title>\n", escape(meta.title)));
    head.push_str(&format!(
        "    <link rel=\"stylesheet\" href=\"{}\">\n",
        escape(meta.stylesheet_relative)
    ));
    head.push_str("</head>\n<body>\n");

    let mut body = String::new();
    if let Some(anchor) = meta.anchor {
        body.push_str(&format!("<a name=\"{}\"></a>\n", escape(anchor)));
    }
    body.push_str("<article class=\"chapter\">\n");
    body.push_str(body_fragment);
    body.push_str("\n</article>\n");
    body.push_str("</body>\n</html>\n");

    format!("{head}{body}")
}

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppleHelpConfig, IndexFormat, IndexOverride};

    fn fragment(md: &str) -> String {
        markdown_to_html_fragment(md)
    }

    /// Sibling-relative `.md` links must be swapped for `.html`, and the
    /// original URL must no longer appear anywhere in the output (otherwise
    /// rewriting "succeeded" but left a stale broken link in place).
    #[test]
    fn rewrites_relative_md_links_to_html() {
        let out = fragment("[next](./next.md)");
        assert!(out.contains(r#"href="./next.html""#), "got: {out}");
        assert!(!out.contains("./next.md"), "stale URL leaked: {out}");
    }

    /// `../` (parent-relative) `.md` links must also be rewritten.
    #[test]
    fn rewrites_parent_relative_md_links() {
        let out = fragment("[back](../intro.md)");
        assert!(out.contains(r#"href="../intro.html""#), "got: {out}");
        assert!(!out.contains("../intro.md"), "stale URL leaked: {out}");
    }

    /// `page.md#section-2` should rewrite the path but keep the anchor —
    /// users use `path#anchor` to jump within rewritten pages.
    #[test]
    fn preserves_fragment_when_rewriting() {
        let out = fragment("[anchored](./page.md#section-2)");
        assert!(out.contains(r#"href="./page.html#section-2""#), "got: {out}");
        assert!(!out.contains("./page.md"), "stale URL leaked: {out}");
    }

    /// `.markdown` (the long-form extension) gets the same treatment as `.md`.
    #[test]
    fn rewrites_markdown_extension_too() {
        let out = fragment("[x](./readme.markdown)");
        assert!(out.contains(r#"href="./readme.html""#), "got: {out}");
        assert!(
            !out.contains("./readme.markdown"),
            "stale URL leaked: {out}"
        );
    }

    /// External URLs that happen to end in `.md` must not be rewritten —
    /// they point at someone else's server, not our bundle. The positive
    /// `contains` already pins the href to the exact original URL.
    #[test]
    fn passes_through_external_https() {
        let out = fragment("[ext](https://example.com/page.md)");
        assert!(
            out.contains(r#"href="https://example.com/page.md""#),
            "got: {out}"
        );
    }

    #[test]
    fn passes_through_mailto() {
        let out = fragment("[mail](mailto:foo@bar.test)");
        assert!(out.contains(r#"href="mailto:foo@bar.test""#), "got: {out}");
    }

    /// An absolute-path link (`/foo/bar.md`) is treated as host-relative
    /// (i.e. user knows what they're doing) and passes through unchanged.
    #[test]
    fn passes_through_absolute_path() {
        let out = fragment("[abs](/absolute/page.md)");
        assert!(out.contains(r#"href="/absolute/page.md""#), "got: {out}");
    }

    /// Fragment-only links (`#section`) navigate within the current page —
    /// they have no file part to rewrite.
    #[test]
    fn passes_through_fragment_only() {
        let out = fragment("[here](#section)");
        assert!(out.contains(r##"href="#section""##), "got: {out}");
    }

    /// Autolinks (`<https://...>` in markdown) are flagged by pulldown-cmark
    /// with `LinkType::Autolink` — we short-circuit those before any rewrite.
    #[test]
    fn passes_through_autolink_url() {
        let out = fragment("<https://example.com>");
        assert!(out.contains(r#"href="https://example.com""#), "got: {out}");
    }

    /// `Tag::Image` URLs (`![alt](src)`) go through the same rewrite path
    /// as `Tag::Link` — verify the image branch is wired correctly.
    #[test]
    fn rewrites_image_src() {
        let out = fragment("![cover](./cover.md)");
        assert!(out.contains(r#"src="./cover.html""#), "got: {out}");
        assert!(!out.contains("./cover.md"), "stale URL leaked: {out}");
    }

    /// Anything that isn't `.md`/`.markdown` (e.g. `.png`, `.pdf`) is left alone.
    #[test]
    fn leaves_non_md_extension_alone() {
        let out = fragment("[img](./pic.png)");
        assert!(out.contains(r#"href="./pic.png""#), "got: {out}");
    }

    /// `is_external` decides whether a URL is "ours to rewrite" or not:
    /// URL schemes, absolute paths, and fragment-only links are external;
    /// bare or `./` / `../` relative paths are ours.
    #[test]
    fn is_external_detects_schemes_paths_and_fragments() {
        assert!(is_external("https://example.com"));
        assert!(is_external("mailto:x@y.test"));
        assert!(is_external("#frag"));
        assert!(is_external("/abs/page.md"));
        assert!(!is_external("./rel.md"));
        assert!(!is_external("rel.md"));
        assert!(!is_external("a/b.md"));
    }

    /// `split_fragment` separates `path#anchor` into `(path, Some(anchor))`,
    /// preserving any `#` characters inside the anchor portion.
    #[test]
    fn split_fragment_separates_path_and_anchor() {
        assert_eq!(split_fragment("a.md"), ("a.md", None));
        assert_eq!(split_fragment("a.md#b"), ("a.md", Some("b")));
        assert_eq!(split_fragment("a.md#b#c"), ("a.md", Some("b#c")));
    }

    fn test_cfg() -> AppleHelpConfig {
        AppleHelpConfig {
            help_book_name: "com.test.help".into(),
            help_book_folder: "TestHelp".into(),
            title: "Test Book".into(),
            description: "A description".into(),
            language: "en".into(),
            authors: vec!["Author One".into(), "Author Two".into()],
            index_format: IndexFormat::Both,
            generate_index: true,
            landing_page: None,
            icon_file: Some("Shared/icon.png".into()),
            external_url: None,
            access_key: None,
            version: "1".into(),
            index_override: IndexOverride::None,
        }
    }

    /// Every Apple Help page needs: language, AppleTitle (the heading shown
    /// in Help Viewer), description/author meta, page <title>, stylesheet link,
    /// and an anchor when one is requested.
    #[test]
    fn render_page_includes_required_meta() {
        let cfg = test_cfg();
        let meta = PageMeta {
            title: "Intro",
            apple_title: "Test Book",
            language: "en",
            stylesheet_relative: "../Shared/style.css",
            anchor: Some("intro"),
        };
        let out = render_page("<p>hi</p>", &meta, &cfg);

        assert!(out.contains(r#"<html lang="en">"#), "lang: {out}");
        assert!(
            out.contains(r#"<meta name="AppleTitle" content="Test Book">"#),
            "AppleTitle: {out}"
        );
        assert!(
            out.contains(r#"<meta name="AppleIcon" content="Shared/icon.png">"#),
            "AppleIcon: {out}"
        );
        assert!(
            out.contains(r#"<meta name="description" content="A description">"#),
            "description: {out}"
        );
        assert!(
            out.contains(r#"<meta name="author" content="Author One, Author Two">"#),
            "author: {out}"
        );
        assert!(out.contains("<title>Intro</title>"), "title: {out}");
        assert!(
            out.contains(r#"<link rel="stylesheet" href="../Shared/style.css">"#),
            "stylesheet: {out}"
        );
        assert!(out.contains(r#"<a name="intro">"#), "anchor: {out}");
        assert!(out.contains("<p>hi</p>"), "body: {out}");
    }

    /// AppleIcon meta is emitted only when `icon_file` is configured.
    #[test]
    fn render_page_omits_apple_icon_when_not_configured() {
        let mut cfg = test_cfg();
        cfg.icon_file = None;
        let meta = PageMeta {
            title: "T",
            apple_title: "T",
            language: "en",
            stylesheet_relative: "../Shared/style.css",
            anchor: Some("x"),
        };
        let out = render_page("x", &meta, &cfg);
        assert!(!out.contains("AppleIcon"), "should omit AppleIcon: {out}");
    }

    /// `PageMeta { anchor: None, .. }` skips the `<a name="...">` element.
    #[test]
    fn render_page_omits_anchor_when_none() {
        let cfg = test_cfg();
        let meta = PageMeta {
            title: "T",
            apple_title: "T",
            language: "en",
            stylesheet_relative: "../Shared/style.css",
            anchor: None,
        };
        let out = render_page("x", &meta, &cfg);
        assert!(!out.contains("<a name="), "should omit anchor: {out}");
    }

    /// User-supplied meta values (description, etc.) must be HTML-escaped
    /// so a stray `<` or `&` in book.toml can't break the document.
    #[test]
    fn render_page_escapes_special_chars_in_meta() {
        let mut cfg = test_cfg();
        cfg.description = "<dangerous & \"quoted\">".into();
        let meta = PageMeta {
            title: "T",
            apple_title: "T",
            language: "en",
            stylesheet_relative: "../Shared/style.css",
            anchor: None,
        };
        let out = render_page("x", &meta, &cfg);
        assert!(out.contains("&lt;dangerous &amp;"), "escape: {out}");
        assert!(out.contains("&quot;quoted&quot;"), "escape quote: {out}");
        assert!(!out.contains("<dangerous"), "raw <: {out}");
    }
}
