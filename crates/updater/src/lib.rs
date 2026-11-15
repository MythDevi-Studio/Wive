//! GitHub Releases update checks and verified self-installation.

use std::{
    env, fs,
    io::{Read, Write},
    path::Path,
    process::{self, Command},
    time::Duration,
};

use semver::Version;
use serde_json::Value;
use sha2::{Digest, Sha256};

const REPOSITORY: &str = "MythDevi-Studio/Wive";
const MAX_DOWNLOAD_SIZE: u64 = 256 * 1024 * 1024;
const USER_AGENT: &str = "Wive-Updater";
const CURRENT_VERSION: &str = include_str!("../../../VERSION");

/// A newer Wive release and its verified release asset metadata.
#[derive(Clone, Debug)]
pub struct UpdateInfo {
    /// Version string from the GitHub release tag.
    pub version: String,
    asset_name: String,
    download_url: String,
    sha256: String,
}

/// Checks the latest stable GitHub release for a compatible desktop binary.
pub fn check_latest() -> Result<Option<UpdateInfo>, String> {
    let asset_name = current_asset_name()
        .ok_or_else(|| "Automatic updates are not available for this platform".to_owned())?;
    let agent = agent();
    let release_url = format!("https://api.github.com/repos/{REPOSITORY}/releases/latest");
    let release = get_json(&agent, &release_url)?;
    let tag = release
        .get("tag_name")
        .and_then(Value::as_str)
        .ok_or_else(|| "GitHub release is missing its tag".to_owned())?;
    let latest = Version::parse(tag.trim_start_matches('v'))
        .map_err(|error| format!("Invalid release version: {error}"))?;
    let current = current_version()?;
    if latest <= current {
        return Ok(None);
    }

    let assets = release
        .get("assets")
        .and_then(Value::as_array)
        .ok_or_else(|| "GitHub release has no assets".to_owned())?;
    let asset_url = asset_download_url(assets, asset_name)?;
    let sums_url = asset_download_url(assets, "SHA256SUMS")?;
    let sums = download(&agent, sums_url, 1024 * 1024)?;
    let checksum = checksum_for(&sums, asset_name)?;
    Ok(Some(UpdateInfo {
        version: tag.to_owned(),
        asset_name: asset_name.to_owned(),
        download_url: asset_url.to_owned(),
        sha256: checksum.to_owned(),
    }))
}

/// Downloads, verifies, installs, and restarts into the new version.
///
/// On success this function replaces the running application and exits the
/// current process after starting the updated executable.
pub fn install_and_restart(update: &UpdateInfo) -> Result<(), String> {
    let agent = agent();
    let bytes = download(&agent, &update.download_url, MAX_DOWNLOAD_SIZE)?;
    if digest(&bytes) != update.sha256 {
        return Err("Update checksum did not match the release manifest".into());
    }

    let executable =
        env::current_exe().map_err(|error| format!("Could not locate Wive: {error}"))?;
    if update.asset_name == "wive-linux-x86_64.deb" {
        #[cfg(target_os = "linux")]
        return install_debian_package(&bytes, &executable);
        #[cfg(not(target_os = "linux"))]
        return Err("This Debian update cannot be installed on this platform".into());
    }

    let parent = executable
        .parent()
        .ok_or_else(|| "Wive executable has no parent directory".to_owned())?;
    let filename = executable
        .file_name()
        .ok_or_else(|| "Wive executable has no file name".to_owned())?
        .to_string_lossy();
    let staging = parent.join(format!(".{filename}.update-{}", process::id()));
    write_staged_binary(&staging, &bytes)?;

    #[cfg(windows)]
    if let Err(error) = install_windows(&executable, &staging) {
        let _ = fs::remove_file(&staging);
        return Err(error);
    }

    #[cfg(unix)]
    if let Err(error) = install_unix(&executable, &staging) {
        let _ = fs::remove_file(&staging);
        return Err(error);
    }

    #[allow(unreachable_code)]
    Err("Automatic restart is not supported on this platform".into())
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(120))
        .build()
}

fn get_json(agent: &ureq::Agent, url: &str) -> Result<Value, String> {
    let response = agent
        .get(url)
        .set("User-Agent", USER_AGENT)
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|error| format!("GitHub release check failed: {error}"))?;
    let mut body = String::new();
    response
        .into_reader()
        .take(1024 * 1024)
        .read_to_string(&mut body)
        .map_err(|error| format!("Could not read GitHub response: {error}"))?;
    serde_json::from_str(&body).map_err(|error| format!("Invalid GitHub response: {error}"))
}

fn asset_download_url<'a>(assets: &'a [Value], name: &str) -> Result<&'a str, String> {
    assets
        .iter()
        .find(|asset| asset.get("name").and_then(Value::as_str) == Some(name))
        .and_then(|asset| asset.get("browser_download_url"))
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Release is missing the {name} asset"))
}

fn download(agent: &ureq::Agent, url: &str, limit: u64) -> Result<Vec<u8>, String> {
    let response = agent
        .get(url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|error| format!("Could not download update: {error}"))?;
    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Could not read update download: {error}"))?;
    if bytes.len() as u64 > limit {
        return Err("Update download exceeds the permitted size".into());
    }
    Ok(bytes)
}

fn checksum_for(manifest: &[u8], asset_name: &str) -> Result<String, String> {
    let manifest = std::str::from_utf8(manifest)
        .map_err(|error| format!("Invalid release checksum manifest: {error}"))?;
    manifest
        .lines()
        .find_map(|line| {
            let mut fields = line.split_whitespace();
            let checksum = fields.next()?;
            let name = fields.next()?.trim_start_matches('*');
            (name == asset_name
                && checksum.len() == 64
                && checksum.bytes().all(|byte| byte.is_ascii_hexdigit()))
            .then(|| checksum.to_ascii_lowercase())
        })
        .ok_or_else(|| format!("Release checksum manifest is missing {asset_name}"))
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn current_version() -> Result<Version, String> {
    Version::parse(CURRENT_VERSION.trim()).map_err(|error| format!("Invalid VERSION file: {error}"))
}

fn current_asset_name() -> Option<&'static str> {
    let executable = env::current_exe().ok()?;
    asset_name_for_platform(env::consts::OS, env::consts::ARCH, &executable)
}

fn asset_name_for_platform(os: &str, arch: &str, executable: &Path) -> Option<&'static str> {
    match (os, arch) {
        ("linux", "x86_64") if is_system_package_path(executable) => Some("wive-linux-x86_64.deb"),
        ("linux", "x86_64") => Some("wive-linux-x86_64"),
        ("windows", "x86_64") => Some("wive-windows-x86_64.exe"),
        ("macos", "x86_64" | "aarch64") => Some("wive-macos-universal"),
        _ => None,
    }
}

fn is_system_package_path(executable: &Path) -> bool {
    executable.starts_with("/usr/bin") || executable.starts_with("/usr/local/bin")
}

fn write_staged_binary(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("Could not stage update beside Wive: {error}"))?;
    let result = (|| {
        file.write_all(bytes)
            .map_err(|error| format!("Could not write staged update: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("Could not flush staged update: {error}"))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o755))
                .map_err(|error| format!("Could not mark update executable: {error}"))?;
        }
        Ok(())
    })();
    if result.is_err() {
        drop(file);
        let _ = fs::remove_file(path);
    }
    result
}

#[cfg(target_os = "linux")]
fn install_debian_package(bytes: &[u8], executable: &Path) -> Result<(), String> {
    let package = env::temp_dir().join(format!("wive-update-{}.deb", process::id()));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&package)
        .map_err(|error| format!("Could not stage Debian update: {error}"))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| {
            let _ = fs::remove_file(&package);
            format!("Could not write Debian update: {error}")
        })?;
    drop(file);

    let status = Command::new("pkexec")
        .args(["dpkg", "--install"])
        .arg(&package)
        .status()
        .map_err(|error| format!("Could not request permission to install Wive: {error}"));
    let _ = fs::remove_file(&package);
    let status = status?;
    if !status.success() {
        return Err(format!("Debian installer exited with status {status}"));
    }

    Command::new(executable)
        .args(env::args_os().skip(1))
        .spawn()
        .map_err(|error| format!("Updated Wive, but could not restart it: {error}"))?;
    process::exit(0);
}

#[cfg(unix)]
fn install_unix(executable: &Path, staging: &Path) -> Result<(), String> {
    fs::rename(staging, executable)
        .map_err(|error| format!("Could not replace Wive (check install permissions): {error}"))?;
    Command::new(executable)
        .args(env::args_os().skip(1))
        .spawn()
        .map_err(|error| format!("Updated Wive, but could not restart it: {error}"))?;
    process::exit(0);
}

#[cfg(windows)]
fn install_windows(executable: &Path, staging: &Path) -> Result<(), String> {
    let script_path = env::temp_dir().join(format!("wive-update-{}.ps1", process::id()));
    let args = env::args_os()
        .skip(1)
        .map(|argument| quote_windows_argument(&argument))
        .collect::<Vec<_>>()
        .join(" ");
    let script = format!(
        "$ErrorActionPreference = 'Stop'\n\
         Wait-Process -Id {} -ErrorAction SilentlyContinue\n\
         Move-Item -LiteralPath {} -Destination {} -Force\n\
         Start-Process -FilePath {} -ArgumentList {}\n\
         Remove-Item -LiteralPath $PSCommandPath -Force\n",
        process::id(),
        powershell_quote(&staging.to_string_lossy()),
        powershell_quote(&executable.to_string_lossy()),
        powershell_quote(&executable.to_string_lossy()),
        powershell_quote(&args),
    );
    if let Err(error) = fs::write(&script_path, script) {
        let _ = fs::remove_file(&script_path);
        return Err(format!("Could not prepare Windows updater: {error}"));
    }
    if let Err(error) = Command::new("powershell.exe")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&script_path)
        .spawn()
    {
        let _ = fs::remove_file(&script_path);
        return Err(format!("Could not start Windows updater: {error}"));
    }
    process::exit(0);
}

#[cfg(windows)]
fn powershell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(windows)]
fn quote_windows_argument(value: &std::ffi::OsStr) -> String {
    let value = value.to_string_lossy();
    let mut quoted = String::from("\"");
    let mut backslashes = 0;
    for character in value.chars() {
        if character == '\\' {
            backslashes += 1;
        } else if character == '"' {
            quoted.push_str(&"\\".repeat(backslashes * 2 + 1));
            quoted.push('"');
            backslashes = 0;
        } else {
            quoted.push_str(&"\\".repeat(backslashes));
            quoted.push(character);
            backslashes = 0;
        }
    }
    quoted.push_str(&"\\".repeat(backslashes * 2));
    quoted.push('"');
    quoted
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{asset_name_for_platform, checksum_for, current_asset_name, current_version};

    #[test]
    fn version_file_contains_valid_semver() {
        assert!(current_version().is_ok());
    }

    #[test]
    fn reads_standard_sha256sum_manifest_lines() {
        let manifest = format!("{}  wive-linux-x86_64\n", "a".repeat(64));
        assert_eq!(
            checksum_for(manifest.as_bytes(), "wive-linux-x86_64").unwrap(),
            "a".repeat(64)
        );
        assert!(checksum_for(manifest.as_bytes(), "wive-windows-x86_64.exe").is_err());
    }

    #[test]
    fn current_platform_has_a_release_asset() {
        if cfg!(all(target_os = "linux", target_arch = "x86_64"))
            || cfg!(all(target_os = "windows", target_arch = "x86_64"))
            || cfg!(all(
                target_os = "macos",
                any(target_arch = "x86_64", target_arch = "aarch64")
            ))
        {
            assert!(current_asset_name().is_some());
        }
    }

    #[test]
    fn selects_debian_installer_for_system_installations() {
        assert_eq!(
            asset_name_for_platform("linux", "x86_64", Path::new("/usr/bin/wive")),
            Some("wive-linux-x86_64.deb")
        );
        assert_eq!(
            asset_name_for_platform("linux", "x86_64", Path::new("/home/user/bin/wive")),
            Some("wive-linux-x86_64")
        );
    }
}
