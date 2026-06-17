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
