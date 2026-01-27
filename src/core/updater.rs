// module responsible for Auto-update functionality (Linux only)
use std::fs;
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
        eprintln!("=== UPDATE CHECK DEBUG ===");
        eprintln!("Repository: {}", self.repo);
        eprintln!("Current version: {}", CURRENT_VERSION);
        
        let client = match reqwest::blocking::Client::builder()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
            .timeout(std::time::Duration::from_secs(10))
            .build() {
                Ok(c) => {
                    eprintln!("✓ HTTP client created");
                    c
                }
                Err(e) => {
                    eprintln!("✗ Failed to create HTTP client: {}", e);
                    return Err(format!("Failed to create HTTP client: {}", e).into());
                }
            };
        
        let release_url = format!("https://api.github.com/repos/{}/releases/latest", self.repo);
        eprintln!("Fetching: {}", release_url);
        
        let response = match client.get(&release_url).send() {
            Ok(r) => {
                eprintln!("✓ Request sent successfully");
                r
            }
            Err(e) => {
                eprintln!("✗ Network error: {}", e);
                return Err(format!("Network error: {}. Check your internet connection.", e).into());
            }
        };
        
        let status = response.status();
        eprintln!("Response status: {} ({})", status, status.as_u16());
        
        if status.as_u16() == 404 {
            eprintln!("✗ 404 Not Found - Repository or releases don't exist");
            return Err("Repository not found or no releases available.\n\
                Repository: DomanskiFilip/quick_notepad\n\
                Make sure releases exist (not just tags)".into());
        }
        
        if !status.is_success() {
            let body = response.text().unwrap_or_else(|_| "Could not read response body".to_string());
            eprintln!("✗ HTTP error. Response body: {}", body);
            return Err(format!("Failed to fetch releases: {} (status: {})\nResponse: {}", 
                status, status.as_u16(), body).into());
        }
        
        eprintln!("✓ Got successful response, parsing JSON...");
        
        let release: GitHubRelease = match response.json() {
            Ok(r) => {
                eprintln!("✓ JSON parsed successfully");
                r
            }
            Err(e) => {
                eprintln!("✗ Failed to parse JSON: {}", e);
                return Err(format!("Failed to parse release data: {}", e).into());
            }
        };
        
        eprintln!("Latest release tag: {}", release.tag_name);
        eprintln!("Number of assets: {}", release.assets.len());
        
        let latest_version = release.tag_name.trim_start_matches('v');
        let current_version = CURRENT_VERSION;
        
        eprintln!("Comparing versions: {} vs {}", current_version, latest_version);
        
        let update_available = Self::is_newer_version(current_version, latest_version);
        
        eprintln!("Update available: {}", update_available);
        eprintln!("=== END DEBUG ===");
        
        Ok(UpdateInfo {
            current_version: current_version.to_string(),
            latest_version: latest_version.to_string(),
            update_available,
            release_notes: release.body.unwrap_or_else(|| 
                format!("Update to version {}", latest_version)
            ),
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

    // Download and install the update (Linux only)
    pub fn perform_update(&self) -> Result<(), Box<dyn std::error::Error>> {
        let client = reqwest::blocking::Client::builder()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
            .timeout(std::time::Duration::from_secs(60))
            .build()?;
        
        let release_url = format!("https://api.github.com/repos/{}/releases/latest", self.repo);
        let response = client.get(&release_url).send()?;
        
        if !response.status().is_success() {
            return Err(format!("Failed to fetch release: {}", response.status()).into());
        }
        
        let release: GitHubRelease = response.json()?;
        
        // Find the Linux binary - look for asset named "quick" (no extension)
        let asset = release.assets
            .iter()
            .find(|a| {
                let name = a.name.to_lowercase();
                // Look for binary named "quick" without extension
                // Skip source archives
                a.name == "quick" || 
                (name.contains("linux") && !name.ends_with(".zip") && !name.ends_with(".tar.gz"))
            })
            .ok_or("No Linux binary found in release")?;
        
        // Download the binary
        let download_response = client.get(&asset.browser_download_url).send()?;
        
        if !download_response.status().is_success() {
            return Err(format!("Failed to download update: {}", download_response.status()).into());
        }
        
        let bytes = download_response.bytes()?;
        
        // Get current executable path
        let current_exe = std::env::current_exe()?;
        let backup_path = current_exe.with_extension("old");
        
        // Create backup of current executable
        fs::copy(&current_exe, &backup_path)?;
        
        // Write new executable to temp location
        let temp_path = current_exe.with_extension("new");
        fs::write(&temp_path, bytes)?;
        
        // Make it executable
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&temp_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&temp_path, perms)?;
        
        // Replace old executable with new one
        fs::rename(&temp_path, &current_exe)?;
        
        // Clean up backup on success
        let _ = fs::remove_file(&backup_path);
        
        Ok(())
    }
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