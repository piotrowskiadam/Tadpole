fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let out_dir = std::env::var("OUT_DIR").unwrap();
        let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap();

        if target_env == "gnu" {
            // Find the GNU resource compiler tool
            let windres = if std::process::Command::new("x86_64-w64-mingw32-windres").arg("--version").status().is_ok() {
                "x86_64-w64-mingw32-windres"
            } else {
                "windres"
            };

            let status = std::process::Command::new(windres)
                .args(&["-i", "tadpole.rc", "-o"])
                .arg(format!("{}/tadpole.o", out_dir))
                .status()
                .expect("Failed to execute windres");

            if !status.success() {
                panic!("windres failed to compile resource file");
            }
            // Pass the object file directly to the linker arguments so it doesn't get stripped
            println!("cargo:rustc-link-arg={}/tadpole.o", out_dir);
        }

        println!("cargo:rerun-if-changed=tadpole.rc");
        println!("cargo:rerun-if-changed=tadpole.ico");
    }
}
