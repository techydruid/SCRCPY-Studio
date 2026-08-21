use crate::runtime::{adb_path, output_text};
use std::process::Command;

fn safe_address(address: &str) -> Result<String, String> {
    let value = address.trim();
    if value.is_empty()
        || value.len() > 255
        || value.chars().any(char::is_whitespace)
        || !value.contains(':')
    {
        return Err("Enter a valid host:port address, for example 192.168.1.20:37123.".into());
    }
    Ok(value.to_string())
}

#[tauri::command]
pub(crate) fn pair_device(address: String, code: String) -> Result<String, String> {
    let address = safe_address(&address)?;
    let code = code.trim();
    if code.len() < 6 || code.len() > 12 || !code.chars().all(|c| c.is_ascii_digit()) {
        return Err("Enter the numeric pairing code shown on the phone.".into());
    }
    let adb = adb_path()?;
    let mut command = Command::new(adb);
    command.args(["pair", &address, code]);
    let output = output_text(command)?;
    Ok(if output.is_empty() {
        "Pairing request completed.".into()
    } else {
        output
    })
}

#[tauri::command]
pub(crate) fn connect_device(address: String) -> Result<String, String> {
    let address = safe_address(&address)?;
    let adb = adb_path()?;
    let mut command = Command::new(adb);
    command.args(["connect", &address]);
    let output = output_text(command)?;
    Ok(if output.is_empty() {
        format!("Connected to {address}")
    } else {
        output
    })
}
