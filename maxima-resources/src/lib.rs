use std::{io, path::PathBuf};

pub fn maxima_windows_rc(internal_name: &str, display_name: &str) -> io::Result<()> {
    if !cfg!(target_os = "windows") {
        return Ok(());
    }

    let assets_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");

    let version = env!("CARGO_PKG_VERSION");
    let license = env!("CARGO_PKG_LICENSE");
    let repository = env!("CARGO_PKG_REPOSITORY");

    let mut res = winres::WindowsResource::new();
    res.set_icon(assets_path.join("logo.ico").to_str().unwrap())
        .set("Comments", &format!("Maxima Game Launcher - {}", repository))
        .set("CompanyName", "Armchair Developers")
        .set("FileDescription", display_name)
        .set("FileVersion", version)
        .set("InternalName", internal_name)
        .set("LegalTrademarks", license)
        .set("ProductName", display_name)
        .set("ProductVersion", version)
        // manually set version 1.0.0.0
        .set_version_info(winres::VersionInfo::PRODUCTVERSION, 0x0001000000000000);
    res.compile()?;

    Ok(())
}