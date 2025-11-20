fn main() {
    // Tell Cargo to rerun build when versions.json changes
    println!("cargo:rerun-if-changed=versions.json");
    
    tauri_build::build()
}
