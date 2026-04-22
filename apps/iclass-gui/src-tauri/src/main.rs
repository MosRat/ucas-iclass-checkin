//! Tauri desktop backend binary entrypoint for the UCAS iCLASS GUI shell.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(feature = "allocator-mimalloc")]
use mimalloc::MiMalloc;

/// Global allocator selection for the desktop GUI binary.
#[cfg(feature = "allocator-mimalloc")]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() {
    iclass_gui_app_runtime::run();
}
