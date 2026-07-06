//! Subprocess construction shared by the git engine and the credential layer.

use std::process::Command;

/// A `Command` that never flashes a console window. On Windows every spawn from
/// a GUI process opens a visible console unless CREATE_NO_WINDOW is set, and the
/// sync engine spawns git once per repo; elsewhere this is a plain `Command`.
pub(crate) fn command(program: &str) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}
