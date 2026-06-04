fn main() {
    #[cfg(feature = "gui")]
    slint_build::compile("ui/app.slint").unwrap();

    // Embed application icon for Windows (taskbar, Alt+Tab, title bar)
    if cfg!(target_os = "windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("icons/icon.ico");
        res.compile().unwrap();
    }
}
