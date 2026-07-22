#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

//! Entry point only. Everything lives in the library (`src/lib.rs`) so that its
//! modules can stay `pub(crate)` and under dead-code analysis — see the module
//! docs there for why that matters.

fn main() -> Result<(), eframe::Error> {
    er_save_reader::run()
}
