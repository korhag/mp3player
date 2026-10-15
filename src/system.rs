// ------------------------------------------------------------
// Module: system
// Purpose: Opens a file in the system file manager, and opens
//          a URL in the default browser.
// ------------------------------------------------------------

use std::io;
use std::path::Path;
use std::process::Command;

// ------------------------------------------------------------
// Function: reveal
// Purpose: Shows a file in Explorer, or its folder on Linux.
//          A missing file falls back to the parent folder.
// ------------------------------------------------------------
pub fn reveal(path: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        if path.exists() {
            Command::new("explorer.exe")
                .raw_arg(format!("/select,\"{}\"", path.display()))
                .spawn()?;
        } else {
            let folder = path.parent().unwrap_or(path);
            Command::new("explorer.exe").arg(folder).spawn()?;
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let folder = if path.is_dir() {
            path
        } else {
            path.parent().unwrap_or(path)
        };
        Command::new("xdg-open").arg(folder).spawn()?;
        Ok(())
    }
}

// ------------------------------------------------------------
// Function: open_url
// Purpose: Opens a web page in the default browser.
// ------------------------------------------------------------
pub fn open_url(url: &str) -> io::Result<()> {
    #[cfg(windows)]
    {
        Command::new("cmd").args(["/C", "start", "", url]).spawn()?;
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(url).spawn()?;
        return Ok(());
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Command::new("xdg-open").arg(url).spawn()?;
        Ok(())
    }
}
