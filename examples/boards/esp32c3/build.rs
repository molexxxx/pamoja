//! Links with the script the chip's HAL ships, which places the program in the ESP32-C3's
//! memory map.

fn main() {
    println!("cargo:rustc-link-arg=-Tlinkall.x");
    println!("cargo:rerun-if-changed=build.rs");
}
