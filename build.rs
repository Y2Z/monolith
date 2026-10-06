fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    #[cfg(feature = "gui")]
    embed_windows_icon();
}

/// Bakes the icon into the .exe, so that Explorer, the taskbar, shortcuts, etc. show it
#[cfg(feature = "gui")]
fn embed_windows_icon() {
    println!("cargo:rerun-if-changed=assets/icon/icon.ico");

    // build.rs runs on the host, so check the *target* OS (works when cross-compiling too)
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let mut resource = winresource::WindowsResource::new();
    resource.set_icon("assets/icon/icon.ico");
    if let Err(error) = resource.compile() {
        println!("cargo:warning=could not embed Windows icon: {error}");
    }
}
