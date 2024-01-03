// Jackson Coxson

fn main() {
    // Change the directory to frontend
    println!("cargo:rerun-if-changed=frontend");
    std::env::set_current_dir("frontend").unwrap();

    // Run the build script
    std::process::Command::new("npm")
        .arg("run")
        .arg("build")
        .status()
        .expect("Failed to build frontend");
}
