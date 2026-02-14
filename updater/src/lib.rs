use colored::Colorize;
use reqwest::Client;
use serde::{ Deserialize, Serialize };
use std::fs;
use std::path::PathBuf;
use std::io::Write;

#[derive(Debug, Serialize, Deserialize)]
struct Release {
    tag_name: String,
    name: String,
    body: String,
    assets: Vec<Asset>,
    draft: bool,
    prerelease: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    let (os, arch) = detect_os_and_arch();
    println!("Detected OS: {} ({})", os.cyan(), arch.cyan());

    let url = "https://api.github.com/repos/Team-Gauntlet/Topaz/releases/latest";

    println!("{}", "Checking for updates...".cyan());

    let response = client.get(url).header("User-Agent", "topaz-updater").send().await?;

    if !response.status().is_success() {
        eprintln!("{}", format!("Failed to fetch releases: {}", response.status()).red());
        return Ok(());
    }

    let release: Release = response.json().await?;

    println!("Latest version: {}", release.tag_name.yellow());
    println!("Release: {}", release.name);
    println!("\nChanges:\n{}", release.body);

    let asset_name = format_asset_name(&os, &arch);
    println!("\nLooking for asset matching: {}", asset_name.green());

    if let Some(asset) = release.assets.iter().find(|a| a.name.contains(&asset_name)) {
        println!("{}", "Found matching binary!".green());
        println!("Download URL: {}", asset.browser_download_url.cyan());

        let download_path = download_binary(&client, asset, &os).await?;
        println!("{}", format!("Downloaded to: {}", download_path.display()).green());

        println!(
            "\n{}",
            format!(
                "Or visit: https://github.com/Team-Gauntlet/Topaz/releases/tag/{}",
                release.tag_name
            ).cyan()
        );

        replace_binary(&download_path, &os)?;
    } else {
        println!("{}", format!("No matching binary found for {} ({})", os, arch).yellow());
        println!("Available assets:");
        for asset in &release.assets {
            println!("  - {}", asset.name);
        }
    }

    Ok(())
}

async fn download_binary(
    client: &Client,
    asset: &Asset,
    os: &str
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let bytes = client.get(&asset.browser_download_url).send().await?.bytes().await?;

    let temp_dir = std::env::temp_dir();
    let file_name = if os == "windows" { "topaz-new.exe" } else { "topaz-new" };
    let download_path = temp_dir.join(file_name);

    let mut file = fs::File::create(&download_path)?;
    file.write_all(&bytes)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&download_path, fs::Permissions::from_mode(0o755))?;
    }

    Ok(download_path)
}

fn replace_binary(new_binary_path: &PathBuf, os: &str) -> Result<(), Box<dyn std::error::Error>> {
    let current_exe = std::env::current_exe()?;
    let exe_dir = current_exe.parent().ok_or("Could not determine exe directory")?;

    let target_exe = if os == "windows" {
        exe_dir.join("topaz.exe")
    } else {
        exe_dir.join("topaz")
    };

    let backup_exe = if os == "windows" {
        exe_dir.join("topaz.exe.bak")
    } else {
        exe_dir.join("topaz.bak")
    };

    if target_exe.exists() {
        fs::rename(&target_exe, &backup_exe)?;
    }

    fs::rename(new_binary_path, &target_exe)?;

    if backup_exe.exists() {
        fs::remove_file(&backup_exe)?;
    }

    println!("{}", "Binary updated successfully!".green());

    Ok(())
}

fn detect_os_and_arch() -> (String, String) {
    let os = if cfg!(target_os = "windows") {
        "windows".to_string()
    } else if cfg!(target_os = "macos") {
        "macos".to_string()
    } else if cfg!(target_os = "linux") {
        "linux".to_string()
    } else {
        "unknown".to_string()
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "x86_64".to_string()
    } else if cfg!(target_arch = "aarch64") {
        "aarch64".to_string()
    } else if cfg!(target_arch = "arm") {
        "arm".to_string()
    } else {
        "unknown".to_string()
    };

    (os, arch)
}

fn format_asset_name(os: &str, arch: &str) -> String {
    match (os, arch) {
        ("windows", "x86_64") => "topaz-windows-x86_64.exe".to_string(),
        ("windows", "aarch64") => "topaz-windows-aarch64.exe".to_string(),
        ("macos", "x86_64") => "topaz-macos-x86_64".to_string(),
        ("macos", "aarch64") => "topaz-macos-aarch64".to_string(),
        ("linux", "x86_64") => "topaz-linux-x86_64".to_string(),
        ("linux", "aarch64") => "topaz-linux-aarch64".to_string(),
        _ => format!("topaz-{}-{}", os, arch),
    }
}
