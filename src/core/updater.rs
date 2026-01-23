// src/core/updater.rs - Auto-update functionality
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

const GITHUB_REPO: &str = "DomanskiFilip/quick_notepad";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
    body: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub release_notes: String,
}

pub struct Updater {
    repo: String,
}

impl Updater {
    pub fn new() -> Self {
        Self {
            repo: GITHUB_REPO.to_string(),
        }
    }

    // Check if an update is available
    pub fn check_for_updates(&self) -> Result<UpdateInfo, Box<dyn std::error::Error>> {
        let url = format!("https://api.github.com/repos/{}/releases/latest", self.repo);
        
        let client = reqwest::blocking::Client::builder()
            .user_agent("quick-notepad")
            .build()?;
        
        let response = client.get(&url).send()?;
        
        if !response.status().is_success() {
            return Err(format!("Failed to fetch releases: {}", response.status()).into());
        }
        
        let release: GitHubRelease = response.json()?;
        let latest_version = release.tag_name.trim_start_matches('v');
        let current_version = CURRENT_VERSION;
        
        let update_available = Self::is_newer_version(current_version, latest_version);
        
        Ok(UpdateInfo {
            current_version: current_version.to_string(),
            latest_version: latest_version.to_string(),
            update_available,
            release_notes: release.body.unwrap_or_default(),
        })
    }

    // Compare version strings (simple semantic versioning)
    fn is_newer_version(current: &str, latest: &str) -> bool {
        let current_parts: Vec<u32> = current
            .split('.')
            .filter_map(|s| s.parse().ok())
            .collect();
        let latest_parts: Vec<u32> = latest
            .split('.')
            .filter_map(|s| s.parse().ok())
            .collect();
        
        for i in 0..3 {
            let c = current_parts.get(i).unwrap_or(&0);
            let l = latest_parts.get(i).unwrap_or(&0);
            
            if l > c {
                return true;
            } else if l < c {
                return false;
            }
        }
        
        false
    }

    // Download and install the update
    pub fn perform_update(&self) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!("https://api.github.com/repos/{}/releases/latest", self.repo);
        
        let client = reqwest::blocking::Client::builder()
            .user_agent("quick-notepad")
            .build()?;
        
        let response = client.get(&url).send()?;
        let release: GitHubRelease = response.json()?;
        
        // Determine the correct asset for the current platform
        let asset = self.find_matching_asset(&release.assets)?;
        
        // Download the asset
        println!("Downloading update from: {}", asset.browser_download_url);
        let download_response = client.get(&asset.browser_download_url).send()?;
        
        if !download_response.status().is_success() {
            return Err("Failed to download update".into());
        }
        
        let bytes = download_response.bytes()?;
        
        // Get current executable path
        let current_exe = std::env::current_exe()?;
        let backup_path = current_exe.with_extension("old");
        
        // Create backup of current executable
        fs::copy(&current_exe, &backup_path)?;
        
        // Write new executable
        let temp_path = current_exe.with_extension("new");
        fs::write(&temp_path, bytes)?;
        
        // Make it executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&temp_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&temp_path, perms)?;
        }
        
        // Replace old executable with new one
        fs::rename(&temp_path, &current_exe)?;
        
        // Clean up backup on success
        let _ = fs::remove_file(&backup_path);
        
        Ok(())
    }

    // Find the correct asset for the current platform
    fn find_matching_asset(&self, assets: &[GitHubAsset]) -> Result<&GitHubAsset, Box<dyn std::error::Error>> {
        let target = Self::get_target_triple();
        
        for asset in assets {
            if asset.name.contains(&target) {
                return Ok(asset);
            }
        }
        
        // Fallback: try to match by platform name
        let platform = if cfg!(target_os = "linux") {
            "linux"
        } else if cfg!(target_os = "macos") {
            "macos"
        } else if cfg!(target_os = "windows") {
            "windows"
        } else {
            return Err("Unsupported platform".into());
        };
        
        for asset in assets {
            if asset.name.to_lowercase().contains(platform) {
                return Ok(asset);
            }
        }
        
        Err("No matching asset found for this platform".into())
    }

    // Get the target triple for the current platform
    fn get_target_triple() -> String {
        if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            "x86_64-unknown-linux-gnu".to_string()
        } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
            "aarch64-unknown-linux-gnu".to_string()
        } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
            "x86_64-apple-darwin".to_string()
        } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            "aarch64-apple-darwin".to_string()
        } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
            "x86_64-pc-windows-msvc".to_string()
        } else {
            env!("TARGET").to_string()
        }
    }
}

// Interactive update prompt for TUI
pub fn prompt_update_tui() -> io::Result<bool> {
    print!("An update is available. Would you like to install it? (y/n): ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    Ok(input.trim().eq_ignore_ascii_case("y"))
}

// Update progress callback
pub fn show_update_progress(message: &str) {
    println!("{}", message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert!(Updater::is_newer_version("1.0.0", "1.0.1"));
        assert!(Updater::is_newer_version("1.0.0", "1.1.0"));
        assert!(Updater::is_newer_version("1.0.0", "2.0.0"));
        assert!(!Updater::is_newer_version("1.0.1", "1.0.0"));
        assert!(!Updater::is_newer_version("1.0.0", "1.0.0"));
    }
}