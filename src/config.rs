use anyhow::{Context, Result, anyhow};
use mdbook_renderer::RenderContext;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexOverride {
    None,
    ForceRun,
    ForceSkip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexFormat {
    CoreSpotlight,
    Lsm,
    Both,
}

impl<'de> Deserialize<'de> for IndexFormat {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        match s.as_str() {
            "corespotlight" => Ok(IndexFormat::CoreSpotlight),
            "lsm" => Ok(IndexFormat::Lsm),
            "both" => Ok(IndexFormat::Both),
            other => Err(serde::de::Error::custom(format!(
                "invalid index-format `{other}` (expected `corespotlight`, `lsm`, or `both`)"
            ))),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
struct RawConfig {
    help_book_name: Option<String>,
    help_book_folder: Option<String>,
    title: Option<String>,
    description: Option<String>,
    language: Option<String>,
    authors: Option<Vec<String>>,
    generate_index: Option<bool>,
    index_format: Option<IndexFormat>,
    landing_page: Option<String>,
    icon_file: Option<String>,
    external_url: Option<String>,
    access_key: Option<String>,
}

#[derive(Debug)]
pub struct AppleHelpConfig {
    pub help_book_name: String,
    pub help_book_folder: String,
    pub title: String,
    pub description: String,
    pub language: String,
    pub authors: Vec<String>,
    pub index_format: IndexFormat,
    pub generate_index: bool,
    pub landing_page: Option<String>,
    pub icon_file: Option<String>,
    pub external_url: Option<String>,
    pub access_key: Option<String>,
    pub index_override: IndexOverride,
}

impl AppleHelpConfig {
    pub fn from_context(ctx: &RenderContext, index_override: IndexOverride) -> Result<Self> {
        let raw: RawConfig = ctx
            .config
            .get::<RawConfig>("output.applehelp")
            .context("reading [output.applehelp]")?
            .unwrap_or_default();

        let help_book_name = raw
            .help_book_name
            .ok_or_else(|| anyhow!("`help-book-name` is required in [output.applehelp]"))?;
        let help_book_folder = raw
            .help_book_folder
            .ok_or_else(|| anyhow!("`help-book-folder` is required in [output.applehelp]"))?;

        let book = &ctx.config.book;
        let title = raw
            .title
            .or_else(|| book.title.clone())
            .unwrap_or_else(|| help_book_folder.clone());
        let description = raw
            .description
            .or_else(|| book.description.clone())
            .unwrap_or_default();
        let language = raw
            .language
            .or_else(|| book.language.clone())
            .unwrap_or_else(|| "en".to_string());
        let authors = raw.authors.unwrap_or_else(|| book.authors.clone());

        Ok(Self {
            help_book_name,
            help_book_folder,
            title,
            description,
            language,
            authors,
            index_format: raw.index_format.unwrap_or(IndexFormat::Both),
            generate_index: raw.generate_index.unwrap_or(true),
            landing_page: raw.landing_page,
            icon_file: raw.icon_file,
            external_url: raw.external_url,
            access_key: raw.access_key,
            index_override,
        })
    }

    pub fn should_generate_index(&self) -> bool {
        match self.index_override {
            IndexOverride::ForceRun => true,
            IndexOverride::ForceSkip => false,
            IndexOverride::None => self.generate_index,
        }
    }

    /// Stem of the index filenames (e.g. `My Book` → `My Book.cshelpindex`).
    pub fn index_stem(&self) -> &str {
        &self.title
    }

    pub fn cshelp_index_filename(&self) -> String {
        format!("{}.cshelpindex", self.index_stem())
    }

    pub fn lsm_index_filename(&self) -> String {
        format!("{}.helpindex", self.index_stem())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mdbook_renderer::book::Book;
    use mdbook_renderer::config::Config;
    use std::path::PathBuf;
    use std::str::FromStr;

    fn ctx_from_toml(toml: &str) -> RenderContext {
        let config = Config::from_str(toml).expect("parse config");
        RenderContext::new(PathBuf::from("/tmp/root"), Book::new(), config, PathBuf::from("/tmp/out"))
    }

    #[test]
    fn errors_when_help_book_name_missing() {
        let ctx = ctx_from_toml(
            r#"
[book]
title = "T"
language = "en"

[output.applehelp]
help-book-folder = "TestHelp"
"#,
        );
        let err = AppleHelpConfig::from_context(&ctx, IndexOverride::None).unwrap_err();
        assert!(
            err.to_string().contains("help-book-name"),
            "msg: {err:#}"
        );
    }

    #[test]
    fn errors_when_help_book_folder_missing() {
        let ctx = ctx_from_toml(
            r#"
[book]
title = "T"
language = "en"

[output.applehelp]
help-book-name = "com.x.help"
"#,
        );
        let err = AppleHelpConfig::from_context(&ctx, IndexOverride::None).unwrap_err();
        assert!(
            err.to_string().contains("help-book-folder"),
            "msg: {err:#}"
        );
    }

    /// With nothing set under `[output.applehelp]`, the title/description/
    /// language/authors should come from the top-level `[book]` table — the
    /// "single source of truth" rule from the spec.
    #[test]
    fn falls_back_to_book_metadata() {
        let ctx = ctx_from_toml(
            r#"
[book]
title = "Book Title"
description = "Book Desc"
language = "fr"
authors = ["A", "B"]

[output.applehelp]
help-book-name = "com.x.help"
help-book-folder = "Folder"
"#,
        );
        let cfg = AppleHelpConfig::from_context(&ctx, IndexOverride::None).unwrap();
        assert_eq!(cfg.title, "Book Title");
        assert_eq!(cfg.description, "Book Desc");
        assert_eq!(cfg.language, "fr");
        assert_eq!(cfg.authors, vec!["A".to_string(), "B".to_string()]);
    }

    /// When a value is set under BOTH `[book]` and `[output.applehelp]`,
    /// the applehelp-table value wins — lets users emit a different title
    /// in the Help Book than the one printed in the rendered HTML book.
    #[test]
    fn applehelp_overrides_take_precedence() {
        let ctx = ctx_from_toml(
            r#"
[book]
title = "Book Title"
language = "en"
authors = ["BookAuthor"]

[output.applehelp]
help-book-name = "com.x.help"
help-book-folder = "Folder"
title = "Override Title"
language = "de"
authors = ["HelpAuthor"]
description = "Override desc"
"#,
        );
        let cfg = AppleHelpConfig::from_context(&ctx, IndexOverride::None).unwrap();
        assert_eq!(cfg.title, "Override Title");
        assert_eq!(cfg.language, "de");
        assert_eq!(cfg.authors, vec!["HelpAuthor".to_string()]);
        assert_eq!(cfg.description, "Override desc");
    }

    #[test]
    fn default_index_format_is_both_and_index_on() {
        let ctx = ctx_from_toml(
            r#"
[book]
title = "T"
language = "en"

[output.applehelp]
help-book-name = "com.x.help"
help-book-folder = "Folder"
"#,
        );
        let cfg = AppleHelpConfig::from_context(&ctx, IndexOverride::None).unwrap();
        assert_eq!(cfg.index_format, IndexFormat::Both);
        assert!(cfg.generate_index);
        assert!(cfg.should_generate_index());
    }

    #[test]
    fn parses_index_format_variants() {
        for (s, want) in [
            ("corespotlight", IndexFormat::CoreSpotlight),
            ("lsm", IndexFormat::Lsm),
            ("both", IndexFormat::Both),
        ] {
            let toml = format!(
                r#"
[book]
title = "T"
language = "en"

[output.applehelp]
help-book-name = "com.x.help"
help-book-folder = "Folder"
index-format = "{s}"
"#,
            );
            let ctx = ctx_from_toml(&toml);
            let cfg = AppleHelpConfig::from_context(&ctx, IndexOverride::None).unwrap();
            assert_eq!(cfg.index_format, want, "for {s}");
        }
    }

    #[test]
    fn rejects_unknown_index_format() {
        let ctx = ctx_from_toml(
            r#"
[book]
title = "T"
language = "en"

[output.applehelp]
help-book-name = "com.x.help"
help-book-folder = "Folder"
index-format = "bogus"
"#,
        );
        let err = AppleHelpConfig::from_context(&ctx, IndexOverride::None).unwrap_err();
        let chain = format!("{err:#}");
        assert!(
            chain.contains("bogus") && chain.contains("index-format"),
            "msg: {chain}"
        );
    }

    /// `--force-index` from the CLI overrides `generate-index = false` in
    /// book.toml — needed for the spec's "run mdbook-applehelp standalone on
    /// macOS after building on CI" recovery workflow.
    #[test]
    fn cli_force_run_beats_config_disabled() {
        let ctx = ctx_from_toml(
            r#"
[book]
title = "T"
language = "en"

[output.applehelp]
help-book-name = "com.x.help"
help-book-folder = "Folder"
generate-index = false
"#,
        );
        let cfg = AppleHelpConfig::from_context(&ctx, IndexOverride::ForceRun).unwrap();
        assert!(cfg.should_generate_index());
    }

    /// `--no-index` skips indexing even when book.toml requests it — used
    /// when invoking the backend on a non-macOS CI box without an `hiutil`.
    #[test]
    fn cli_force_skip_beats_config_enabled() {
        let ctx = ctx_from_toml(
            r#"
[book]
title = "T"
language = "en"

[output.applehelp]
help-book-name = "com.x.help"
help-book-folder = "Folder"
generate-index = true
"#,
        );
        let cfg = AppleHelpConfig::from_context(&ctx, IndexOverride::ForceSkip).unwrap();
        assert!(!cfg.should_generate_index());
    }

    #[test]
    fn index_filenames_derive_from_title() {
        let ctx = ctx_from_toml(
            r#"
[book]
title = "My Book"
language = "en"

[output.applehelp]
help-book-name = "com.x.help"
help-book-folder = "Folder"
"#,
        );
        let cfg = AppleHelpConfig::from_context(&ctx, IndexOverride::None).unwrap();
        assert_eq!(cfg.cshelp_index_filename(), "My Book.cshelpindex");
        assert_eq!(cfg.lsm_index_filename(), "My Book.helpindex");
    }

    #[test]
    fn language_defaults_to_en_when_book_lang_missing() {
        // BookConfig::default() actually sets language = Some("en"), so this just
        // checks the en fallback survives an explicit absence in the applehelp table.
        let ctx = ctx_from_toml(
            r#"
[output.applehelp]
help-book-name = "com.x.help"
help-book-folder = "Folder"
"#,
        );
        let cfg = AppleHelpConfig::from_context(&ctx, IndexOverride::None).unwrap();
        assert_eq!(cfg.language, "en");
    }
}
