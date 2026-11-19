use std::io::Read;
use std::path::PathBuf;

use serde_json::Value;
use sha2::{Digest, Sha256};

const API: &str = "https://api.github.com/repos/Sonhoccode/GoodbyeDPI/releases/latest";
const ASSET_PREFIX: &str = "GoodbyeDPI-Manager-All-in-One-";

#[derive(Clone)]
pub struct Release {
    pub version: String,
    pub url: String,
    pub checksum: String,
}

fn version_number(version: &str) -> Option<(u32, u32)> {
    let version = version.trim_start_matches('v');
    let mut parts = version.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    if parts.next().is_some() { return None; }
    Some((major, minor))
}

pub fn check() -> Result<Option<Release>, String> {
    let value: Value = ureq::get(API)
        .set("User-Agent", "GoodbyeDPI-Manager")
        .timeout(std::time::Duration::from_secs(12))
        .call().map_err(|error| error.to_string())?
        .into_json().map_err(|error| error.to_string())?;
    let version = value["tag_name"].as_str().ok_or("Release tag missing")?;
    if version_number(version).is_none_or(|number| number <= (1, 4)) { return Ok(None); }
    let asset = value["assets"].as_array().and_then(|assets| assets.iter().find(|asset| {
        asset["name"].as_str().is_some_and(|name| name.starts_with(ASSET_PREFIX) && name.ends_with(".exe"))
    })).ok_or("Installer asset missing")?;
    let url = asset["browser_download_url"].as_str().ok_or("Download URL missing")?;
    if !url.starts_with("https://github.com/Sonhoccode/GoodbyeDPI/releases/download/") {
        return Err("Unexpected download host".into());
    }
    let body = value["body"].as_str().ok_or("Release checksum missing")?;
    let checksum = body.lines().find_map(|line| line.trim().strip_prefix("SHA256: "))
        .filter(|hash| hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or("Release checksum missing")?;
    Ok(Some(Release { version: version.into(), url: url.into(), checksum: checksum.into() }))
}

pub fn download(release: &Release) -> Result<PathBuf, String> {
    let mut data = Vec::new();
    ureq::get(&release.url).timeout(std::time::Duration::from_secs(60))
        .call().map_err(|error| error.to_string())?.into_reader()
        .take(100_000_000).read_to_end(&mut data).map_err(|error| error.to_string())?;
    let actual = format!("{:x}", Sha256::digest(&data));
    if !actual.eq_ignore_ascii_case(&release.checksum) { return Err("Installer checksum mismatch".into()); }
    let path = std::env::temp_dir().join(format!("GoodbyeDPI-Manager-{}.exe", release.version));
    std::fs::write(&path, data).map_err(|error| error.to_string())?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_versions() {
        assert_eq!(version_number("v1.01"), Some((1, 1)));
        assert_eq!(version_number("v1.00"), Some((1, 0)));
        assert_eq!(version_number("bad"), None);
    }
}
