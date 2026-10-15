// ------------------------------------------------------------
// Build script
// Purpose: Embeds the application icon and version resource into the
//          Windows executable. Cargo.toml is the version source.
// ------------------------------------------------------------

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=Cargo.toml");

    #[cfg(windows)]
    {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/icon.ico");
        resource
            .compile()
            .expect("embed Windows icon and version resource");
    }
}
