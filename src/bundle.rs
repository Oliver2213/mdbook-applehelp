use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use plist::{Dictionary, Value};

use crate::config::AppleHelpConfig;

pub fn prepare_dirs(bundle_root: &Path, lproj: &Path, shared: &Path) -> Result<()> {
    if bundle_root.exists() {
        fs::remove_dir_all(bundle_root)
            .with_context(|| format!("clearing existing bundle {}", bundle_root.display()))?;
    }
    fs::create_dir_all(bundle_root.join("Contents"))?;
    fs::create_dir_all(lproj)?;
    fs::create_dir_all(shared)?;
    Ok(())
}

pub fn write_info_plist(bundle_root: &Path, cfg: &AppleHelpConfig) -> Result<()> {
    let mut dict = Dictionary::new();
    dict.insert(
        "CFBundleIdentifier".into(),
        Value::String(cfg.help_book_name.clone()),
    );
    dict.insert("CFBundleName".into(), Value::String(cfg.title.clone()));
    dict.insert(
        "CFBundleVersion".into(),
        Value::String(cfg.version.clone()),
    );
    dict.insert(
        "CFBundleShortVersionString".into(),
        Value::String(cfg.version.clone()),
    );
    dict.insert(
        "CFBundleInfoDictionaryVersion".into(),
        Value::String("6.0".into()),
    );
    dict.insert("CFBundleDevelopmentRegion".into(), Value::String(cfg.language.clone()));
    dict.insert("CFBundlePackageType".into(), Value::String("BNDL".into()));

    dict.insert(
        "CFBundleHelpBookFolder".into(),
        Value::String(cfg.help_book_folder.clone()),
    );
    dict.insert(
        "CFBundleHelpBookName".into(),
        Value::String(cfg.help_book_name.clone()),
    );

    dict.insert(
        "HPDBookCSIndexPath".into(),
        Value::String(cfg.cshelp_index_filename()),
    );
    dict.insert(
        "HPDBookLSIndexPath".into(),
        Value::String(cfg.lsm_index_filename()),
    );
    dict.insert("HPDBookIndexPathType".into(), Value::String("0".into()));

    if let Some(icon) = &cfg.icon_file {
        dict.insert("HPDBookIconPath".into(), Value::String(icon.clone()));
    }
    if let Some(url) = &cfg.external_url {
        dict.insert("HPDBookRemoteURL".into(), Value::String(url.clone()));
    }
    if let Some(key) = &cfg.access_key {
        dict.insert("HPDBookAccessKey".into(), Value::String(key.clone()));
    }

    let plist_path = bundle_root.join("Contents").join("Info.plist");
    Value::Dictionary(dict)
        .to_file_xml(&plist_path)
        .with_context(|| format!("writing {}", plist_path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{IndexFormat, IndexOverride};

    fn cfg() -> AppleHelpConfig {
        AppleHelpConfig {
            help_book_name: "com.example.help".into(),
            help_book_folder: "MyHelp".into(),
            title: "My Book".into(),
            description: "Desc".into(),
            language: "en".into(),
            authors: vec!["A".into()],
            index_format: IndexFormat::Both,
            generate_index: true,
            landing_page: None,
            icon_file: Some("Shared/icon.png".into()),
            external_url: Some("https://example.com/help".into()),
            access_key: Some("help".into()),
            version: "2.4.1".into(),
            index_override: IndexOverride::None,
        }
    }

    fn read_string(d: &Dictionary, key: &str) -> Option<String> {
        d.get(key).and_then(|v| v.as_string()).map(str::to_owned)
    }

    /// Write the plist with every optional field populated, then parse it
    /// back via the `plist` crate and confirm each spec-required key carries
    /// the value we put in. A round-trip catches both writer bugs and any
    /// future schema drift.
    #[test]
    fn info_plist_contains_required_keys() {
        let tmp = tempfile::tempdir().unwrap();
        let bundle = tmp.path().join("MyHelp.help");
        let lproj = bundle.join("Contents/Resources/en.lproj");
        let shared = bundle.join("Contents/Resources/Shared");
        prepare_dirs(&bundle, &lproj, &shared).unwrap();
        let c = cfg();
        write_info_plist(&bundle, &c).unwrap();

        let value = plist::Value::from_file(bundle.join("Contents/Info.plist")).unwrap();
        let dict = value.into_dictionary().expect("dict");

        assert_eq!(read_string(&dict, "CFBundleIdentifier").as_deref(), Some("com.example.help"));
        assert_eq!(read_string(&dict, "CFBundleName").as_deref(), Some("My Book"));
        assert_eq!(read_string(&dict, "CFBundleVersion").as_deref(), Some("2.4.1"));
        assert_eq!(
            read_string(&dict, "CFBundleShortVersionString").as_deref(),
            Some("2.4.1"),
        );
        assert_eq!(read_string(&dict, "CFBundleHelpBookFolder").as_deref(), Some("MyHelp"));
        assert_eq!(read_string(&dict, "CFBundleHelpBookName").as_deref(), Some("com.example.help"));
        assert_eq!(read_string(&dict, "CFBundleDevelopmentRegion").as_deref(), Some("en"));
        assert_eq!(read_string(&dict, "CFBundlePackageType").as_deref(), Some("BNDL"));
        assert_eq!(read_string(&dict, "HPDBookCSIndexPath").as_deref(), Some("My Book.cshelpindex"));
        assert_eq!(read_string(&dict, "HPDBookLSIndexPath").as_deref(), Some("My Book.helpindex"));
        assert_eq!(read_string(&dict, "HPDBookIndexPathType").as_deref(), Some("0"));
        assert_eq!(read_string(&dict, "HPDBookIconPath").as_deref(), Some("Shared/icon.png"));
        assert_eq!(read_string(&dict, "HPDBookRemoteURL").as_deref(), Some("https://example.com/help"));
        assert_eq!(read_string(&dict, "HPDBookAccessKey").as_deref(), Some("help"));
    }

    /// Optional keys (icon, remote URL, access key) must NOT be emitted as
    /// empty strings when unset — Help Viewer treats an empty
    /// `HPDBookRemoteURL` differently than a missing one.
    #[test]
    fn info_plist_omits_optional_keys_when_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let bundle = tmp.path().join("MyHelp.help");
        let lproj = bundle.join("Contents/Resources/en.lproj");
        let shared = bundle.join("Contents/Resources/Shared");
        prepare_dirs(&bundle, &lproj, &shared).unwrap();
        let mut c = cfg();
        c.icon_file = None;
        c.external_url = None;
        c.access_key = None;
        write_info_plist(&bundle, &c).unwrap();

        let value = plist::Value::from_file(bundle.join("Contents/Info.plist")).unwrap();
        let dict = value.into_dictionary().expect("dict");

        assert!(!dict.contains_key("HPDBookIconPath"));
        assert!(!dict.contains_key("HPDBookRemoteURL"));
        assert!(!dict.contains_key("HPDBookAccessKey"));
    }

    /// Repeated builds must not leak files from a previous run — a stale
    /// `.html` from a deleted chapter would still get picked up by `hiutil`.
    #[test]
    fn prepare_dirs_clears_existing_bundle() {
        let tmp = tempfile::tempdir().unwrap();
        let bundle = tmp.path().join("MyHelp.help");
        std::fs::create_dir_all(&bundle).unwrap();
        std::fs::write(bundle.join("stale-file"), "old").unwrap();

        let lproj = bundle.join("Contents/Resources/en.lproj");
        let shared = bundle.join("Contents/Resources/Shared");
        prepare_dirs(&bundle, &lproj, &shared).unwrap();

        assert!(!bundle.join("stale-file").exists());
        assert!(lproj.exists());
        assert!(shared.exists());
    }
}
