mod creator;
mod devices;
mod doctor;
mod models;
mod preferences;
mod runtime;
mod session;
mod wireless;

use creator::{capture_screenshot, open_media_folder, open_recordings_folder};
use devices::{inspect_device, list_devices, recommend_settings};
use doctor::run_doctor;
use runtime::{install_official_runtime, runtime_status};
use session::launch_session;
use wireless::{
    connect_device,
    enable_usb_wireless,
    forget_wireless_device,
    list_remembered_wireless,
    pair_device,
    reconnect_wireless_device,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            runtime_status,
            install_official_runtime,
            list_devices,
            inspect_device,
            recommend_settings,
            pair_device,
            connect_device,
            list_remembered_wireless,
            reconnect_wireless_device,
            forget_wireless_device,
            enable_usb_wireless,
            capture_screenshot,
            open_media_folder,
            open_recordings_folder,
            launch_session,
            run_doctor
        ])
        .run(tauri::generate_context!())
        .expect("error while running SCRCPY Studio");
}
