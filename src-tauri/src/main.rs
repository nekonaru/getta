// Release builds are a Windows GUI app: no console window behind the app,
// and closing the (nonexistent) console cannot kill the app. Debug builds
// keep the console so logs are visible during development.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    getta_lib::run();
}
