use crate::models::RuntimeStatus;
use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
};

fn executable_names(base: &str) -> Vec<OsString> {
    #[cfg(target_os = "windows")]
    {
        vec![OsString::from(format!("{base}.exe")), OsString::from(base)]
    }
    #[cfg(not(target_os = "windows"))]
    {
        vec![OsString::from(base)]
    }
}

fn managed_runtime_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|base| base.join("SCRCPY Studio").join("runtime"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        env::var_os("HOME")
            .map(PathBuf::from)
            .map(|base| base.join(".local").join("share").join("scrcpy-studio").join("runtime"))
    }
}

pub(crate) fn resolve_binary(base: &str) -> Option<PathBuf> {
    let names = executable_names(base);
    let mut folders = Vec::new();

    if let Some(managed) = managed_runtime_dir() {
        folders.push(managed);
    }

    if let Ok(current_exe) = env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            folders.extend([
                parent.to_path_buf(),
                parent.join("runtime"),
                parent.join("scrcpy"),
            ]);
        }
    }

    for folder in folders {
        for name in &names {
            let candidate = folder.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    if let Some(paths) = env::var_os("PATH") {
        for folder in env::split_paths(&paths) {
            for name in &names {
                let candidate = folder.join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

pub(crate) fn output_text(mut command: Command) -> Result<String, String> {
    let output = command.output().map_err(|e| e.to_string())?;
    let mut text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        text = String::from_utf8_lossy(&output.stderr).trim().to_string();
    }
    if output.status.success() {
        Ok(text)
    } else if text.is_empty() {
        Err(format!("Command failed with {}", output.status))
    } else {
        Err(text)
    }
}

fn tool_version(path: &Path, arg: &str) -> Option<String> {
    let mut command = Command::new(path);
    command.arg(arg);
    output_text(command)
        .ok()
        .and_then(|text| text.lines().next().map(str::to_owned))
}

#[tauri::command]
pub(crate) fn runtime_status() -> RuntimeStatus {
    let adb = resolve_binary("adb");
    let scrcpy = resolve_binary("scrcpy");
    RuntimeStatus {
        adb_found: adb.is_some(),
        scrcpy_found: scrcpy.is_some(),
        adb_path: adb.as_ref().map(|p| p.display().to_string()),
        scrcpy_path: scrcpy.as_ref().map(|p| p.display().to_string()),
        adb_version: adb.as_ref().and_then(|p| tool_version(p, "version")),
        scrcpy_version: scrcpy.as_ref().and_then(|p| tool_version(p, "--version")),
    }
}

#[tauri::command]
pub(crate) fn install_official_runtime() -> Result<RuntimeStatus, String> {
    #[cfg(not(target_os = "windows"))]
    {
        Err("Automatic runtime installation is currently available on Windows only.".into())
    }

    #[cfg(target_os = "windows")]
    {
        let runtime_dir = managed_runtime_dir().ok_or_else(|| {
            "Windows did not provide a Local AppData folder for the managed runtime.".to_string()
        })?;
        std::fs::create_dir_all(&runtime_dir)
            .map_err(|e| format!("Could not create the runtime folder: {e}"))?;

        let script = r#"
$ErrorActionPreference = 'Stop'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
$runtimeDir = $env:SCRCPY_STUDIO_RUNTIME_DIR
if ([string]::IsNullOrWhiteSpace($runtimeDir)) { throw 'Runtime destination was not provided.' }
$workDir = Join-Path $env:TEMP ('scrcpy-studio-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $workDir -Force | Out-Null
try {
  $headers = @{ 'User-Agent' = 'SCRCPY-Studio' }
  $release = Invoke-RestMethod -Uri 'https://api.github.com/repos/Genymobile/scrcpy/releases/latest' -Headers $headers
  $asset = $release.assets | Where-Object { $_.name -match '^scrcpy-win64-v[0-9].*\.zip$' } | Select-Object -First 1
  if (-not $asset) { throw 'The official Windows 64-bit scrcpy package was not found in the latest release.' }
  $sumAsset = $release.assets | Where-Object { $_.name -eq 'SHA256SUMS.txt' } | Select-Object -First 1
  if (-not $sumAsset) { throw 'The official SHA256SUMS.txt file was not found in the latest release.' }

  $zipPath = Join-Path $workDir $asset.name
  $sumPath = Join-Path $workDir 'SHA256SUMS.txt'
  Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $zipPath -Headers $headers
  Invoke-WebRequest -Uri $sumAsset.browser_download_url -OutFile $sumPath -Headers $headers

  $sumLine = Get-Content $sumPath | Where-Object { $_ -match ([regex]::Escape($asset.name) + '$') } | Select-Object -First 1
  if (-not $sumLine) { throw 'A SHA-256 checksum for the Windows package was not found.' }
  $expected = ($sumLine -split '\s+')[0].ToLowerInvariant()
  $actual = (Get-FileHash -Path $zipPath -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($expected -ne $actual) { throw 'The downloaded scrcpy package failed SHA-256 verification.' }

  $extractDir = Join-Path $workDir 'extract'
  Expand-Archive -Path $zipPath -DestinationPath $extractDir -Force
  $scrcpyExe = Get-ChildItem -Path $extractDir -Filter 'scrcpy.exe' -Recurse -File | Select-Object -First 1
  if (-not $scrcpyExe) { throw 'scrcpy.exe was not found after extracting the official package.' }

  New-Item -ItemType Directory -Path $runtimeDir -Force | Out-Null
  Get-ChildItem -Path $runtimeDir -Force -ErrorAction SilentlyContinue | Remove-Item -Recurse -Force
  Copy-Item -Path (Join-Path $scrcpyExe.Directory.FullName '*') -Destination $runtimeDir -Recurse -Force

  if (-not (Test-Path (Join-Path $runtimeDir 'scrcpy.exe'))) { throw 'scrcpy.exe was not installed correctly.' }
  if (-not (Test-Path (Join-Path $runtimeDir 'adb.exe'))) { throw 'adb.exe was not included in the installed runtime.' }
  Write-Output ('Installed official scrcpy ' + $release.tag_name)
}
finally {
  Remove-Item -Path $workDir -Recurse -Force -ErrorAction SilentlyContinue
}
"#;

        let mut command = Command::new("powershell.exe");
        command
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                script,
            ])
            .env("SCRCPY_STUDIO_RUNTIME_DIR", &runtime_dir);

        output_text(command).map_err(|e| format!("Runtime installation failed: {e}"))?;
        let status = runtime_status();
        if !status.scrcpy_found || !status.adb_found {
            return Err("The runtime download completed, but SCRCPY Studio could not verify scrcpy and ADB.".into());
        }
        Ok(status)
    }
}

pub(crate) fn adb_path() -> Result<PathBuf, String> {
    resolve_binary("adb").ok_or_else(|| {
        "ADB was not found. Use Install official runtime in SCRCPY Studio, install Android Platform Tools, or place adb in the runtime folder."
            .into()
    })
}

pub(crate) fn scrcpy_path() -> Result<PathBuf, String> {
    resolve_binary("scrcpy").ok_or_else(|| {
        "scrcpy was not found. Use Install official runtime in SCRCPY Studio, install official scrcpy, or place it in the runtime folder."
            .into()
    })
}
