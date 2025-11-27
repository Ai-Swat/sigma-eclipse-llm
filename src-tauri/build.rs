fn main() {
    // Tell Cargo to rerun build when versions.json changes
    println!("cargo:rerun-if-changed=versions.json");
    
    // Load .env file from project root
    println!("cargo:rerun-if-changed=../.env");
    
    // Try to load .env file
    let env_path = std::path::Path::new("../.env");
    if env_path.exists() {
        dotenvy::from_path(env_path).ok();
    }
    
    // Pass EXTENSION_ID to the build
    let extension_id = std::env::var("EXTENSION_ID")
        .unwrap_or_else(|_| "lidcgfpdpjpeambpilgmllbefcikkglh".to_string());
    println!("cargo:rustc-env=EXTENSION_ID={}", extension_id);
    
    tauri_build::build()
}
