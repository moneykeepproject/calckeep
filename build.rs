fn main() {
    slint_build::compile("ui/app.slint").expect("Slint build failed");

    // Embed assets/icon.ico as the .exe icon when building on/for Windows
    // (shown in Explorer, the taskbar and Alt-Tab).
    #[cfg(windows)]
    {
        if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
            println!("cargo:rerun-if-changed=assets/icon.ico");
            let mut res = winresource::WindowsResource::new();
            res.set_icon("assets/icon.ico");
            res.set("ProductName", "CalcKeep");
            res.set("FileDescription", "CalcKeep financial calculators");
            // Windows wants four numeric parts: 0.1.0-alpha.1 -> 0.1.0.1 (the
            // pre-release number goes last; a plain release gets .0).
            // Each part is 16 bits, so a nightly's date stamp is dropped.
            let part = |name: &str| -> u64 {
                std::env::var(name)
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0)
                    .min(0xffff)
            };
            let build = std::env::var("CARGO_PKG_VERSION_PRE")
                .ok()
                .and_then(|pre| pre.rsplit('.').next().and_then(|n| n.parse::<u64>().ok()))
                .filter(|n| *n <= 0xffff)
                .unwrap_or(0);
            let (major, minor, patch) = (
                part("CARGO_PKG_VERSION_MAJOR"),
                part("CARGO_PKG_VERSION_MINOR"),
                part("CARGO_PKG_VERSION_PATCH"),
            );
            let four = format!("{major}.{minor}.{patch}.{build}");
            let packed = (major << 48) | (minor << 32) | (patch << 16) | build;
            res.set_version_info(winresource::VersionInfo::FILEVERSION, packed);
            res.set_version_info(winresource::VersionInfo::PRODUCTVERSION, packed);
            res.set("FileVersion", &four);
            res.set(
                "ProductVersion",
                &std::env::var("CARGO_PKG_VERSION").unwrap_or_default(),
            );
            if let Err(e) = res.compile() {
                println!("cargo:warning=could not embed Windows icon: {e}");
            }
        }
    }
}
