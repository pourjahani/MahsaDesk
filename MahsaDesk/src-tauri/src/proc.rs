//! Child-process hygiene.
//!
//! On Windows every `Command::spawn` of a console binary (xray, sing-box,
//! taskkill, powershell, version probes...) opens a console window unless
//! `CREATE_NO_WINDOW` is set. Every spawn site in this crate goes through
//! [`hide`] so the app never flashes cmd windows.

use std::process::Command;

/// Configure `cmd` so it never flashes a console window, and return it for
/// chaining. No-op on non-Windows platforms.
pub fn hide(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: spawn the child with no console window at all.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Run a command to completion with stdio captured and no console window.
pub fn output(mut cmd: Command) -> std::io::Result<std::process::Output> {
    hide(&mut cmd).output()
}
