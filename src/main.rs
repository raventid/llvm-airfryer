use dialoguer::{Select, theme::ColorfulTheme};
use std::path::PathBuf;
use std::process::Command;

const COMPILER_EXPLORER_REPO: &str = "git@github.com:compiler-explorer/compiler-explorer.git";

const MENU_ITEMS: &[&str] = &[
    "Download Compiler Explorer",
    "Exit",
];

fn project_root() -> PathBuf {
    let output = Command::new("cargo")
        .args(["locate-project", "--message-format=plain"])
        .output()
        .expect("failed to run `cargo locate-project`");
    let cargo_toml = String::from_utf8(output.stdout)
        .expect("invalid utf-8 from cargo locate-project");
    PathBuf::from(cargo_toml.trim())
        .parent()
        .expect("Cargo.toml has no parent directory")
        .to_path_buf()
}

fn download_compiler_explorer() {
    let dest = project_root().join("bin").join("compiler-explorer");

    if dest.exists() {
        println!("Compiler Explorer already exists at {}", dest.display());
        println!("Pulling latest changes...");
        let status = Command::new("git")
            .args(["pull"])
            .current_dir(&dest)
            .status()
            .expect("failed to run git pull");
        if !status.success() {
            eprintln!("git pull failed");
            std::process::exit(1);
        }
        println!("Compiler Explorer updated successfully.");
        return;
    }

    let bin_dir = dest.parent().unwrap();
    std::fs::create_dir_all(bin_dir).expect("failed to create bin/ directory");

    println!("Cloning Compiler Explorer into {}...", dest.display());
    let status = Command::new("git")
        .args(["clone", "--depth=1", COMPILER_EXPLORER_REPO])
        .arg(&dest)
        .status()
        .expect("failed to run git clone");

    if !status.success() {
        eprintln!("git clone failed");
        std::process::exit(1);
    }
    println!("Compiler Explorer downloaded successfully.");
}

fn main() {
    println!("🔥 LLVM Airfryer — LLVM compiler framework development toolkit\n");

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("What would you like to do?")
        .items(MENU_ITEMS)
        .default(0)
        .interact()
        .expect("failed to render interactive menu");

    match selection {
        0 => download_compiler_explorer(),
        1 => {
            println!("Goodbye!");
            std::process::exit(0);
        }
        _ => unreachable!(),
    }
}
