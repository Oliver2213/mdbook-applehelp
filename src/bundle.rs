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
    dict.insert("CFBundleVersion".into(), Value::String("1".into()));
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
