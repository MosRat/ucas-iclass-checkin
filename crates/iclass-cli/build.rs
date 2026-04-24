//! Build script for embedding the Windows executable icon.

fn main() {
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rerun-if-changed=windows-icon.rc");
        println!("cargo:rerun-if-changed=../../apps/iclass-gui/src-tauri/icons/icon.ico");
        let _ = embed_resource::compile("windows-icon.rc", embed_resource::NONE);
    }
}
