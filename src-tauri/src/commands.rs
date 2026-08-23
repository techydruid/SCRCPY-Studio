use std::{ffi::OsStr, process::Command};

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Creates a child process without briefly flashing a console window on Windows.
///
/// The flag only controls the console host. GUI programs such as scrcpy and
/// Explorer still show their normal windows.
pub(crate) fn hidden_command<S: AsRef<OsStr>>(program: S) -> Command {
    let mut command = Command::new(program);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    command
}

#[cfg(test)]
mod tests {
    #[test]
    fn preserves_the_requested_program() {
        assert_eq!(super::hidden_command("adb").get_program(), "adb");
    }
}
