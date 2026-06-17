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
