use dialoguer::{Input, Select, theme::ColorfulTheme};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;

const COMPILER_EXPLORER_REPO: &str = "git@github.com:compiler-explorer/compiler-explorer.git";
const CE_DEFAULT_PORT: u16 = 10240;

const MENU_ITEMS: &[&str] = &[
    "Download Compiler Explorer",
    "Run Compiler Explorer",
    "Build LLVM Upstream",
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

fn find_available_port(start: u16) -> Option<u16> {
    (start..=u16::MAX).find(|&port| TcpListener::bind(("127.0.0.1", port)).is_ok())
}

fn run_compiler_explorer() {
    let ce_dir = project_root().join("bin").join("compiler-explorer");

    if !ce_dir.exists() {
        eprintln!("Compiler Explorer not found. Please download it first.");
        std::process::exit(1);
    }

    let node_modules = ce_dir.join("node_modules");
    if !node_modules.exists() {
        println!("Installing npm dependencies (first run)...");
        let status = Command::new("npm")
            .arg("install")
            .current_dir(&ce_dir)
            .status()
            .expect("failed to run npm install");
        if !status.success() {
            eprintln!("npm install failed");
            std::process::exit(1);
        }
    }

    let port = find_available_port(CE_DEFAULT_PORT).unwrap_or_else(|| {
        eprintln!("No available port found starting from {CE_DEFAULT_PORT}");
        std::process::exit(1);
    });

    println!("Starting Compiler Explorer on http://localhost:{port} ...");

    let status = Command::new("make")
        .arg("dev")
        .env("EXTRA_ARGS", format!("--port {port}"))
        .current_dir(&ce_dir)
        .status()
        .expect("failed to start Compiler Explorer");

    if !status.success() {
        eprintln!("Compiler Explorer exited with an error");
        std::process::exit(1);
    }
}

fn run_cmd(cmd: &str, args: &[&str], dir: &PathBuf, context: &str) {
    let status = Command::new(cmd)
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap_or_else(|e| panic!("failed to run {context}: {e}"));
    if !status.success() {
        eprintln!("{context} failed");
        std::process::exit(1);
    }
}

fn has_ninja() -> bool {
    Command::new("ninja")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn build_llvm_upstream() {
    let llvm_path: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Path to llvm-project directory")
        .interact_text()
        .expect("failed to read input");

    let llvm_dir = PathBuf::from(shellexpand::tilde(llvm_path.trim()).into_owned())
        .canonicalize()
        .unwrap_or_else(|_| {
            eprintln!("Path does not exist: {llvm_path}");
            std::process::exit(1);
        });

    if !llvm_dir.join("llvm").exists() {
        eprintln!(
            "Invalid llvm-project directory: expected 'llvm/' subdirectory in {}",
            llvm_dir.display()
        );
        std::process::exit(1);
    }

    // Switch to main and pull latest
    println!("Switching to main branch...");
    run_cmd("git", &["checkout", "main"], &llvm_dir, "git checkout main");

    println!("Pulling latest changes...");
    run_cmd("git", &["pull"], &llvm_dir, "git pull");

    let install_dir = project_root().join("bin").join("llvm-upstream");
    let build_dir = llvm_dir.join("build-airfryer");

    std::fs::create_dir_all(&build_dir).expect("failed to create build directory");
    std::fs::create_dir_all(&install_dir).expect("failed to create install directory");

    let generator = if has_ninja() { "Ninja" } else { "Unix Makefiles" };

    println!("Configuring LLVM (generator: {generator})...");
    let install_prefix = format!("-DCMAKE_INSTALL_PREFIX={}", install_dir.display());
    let llvm_src = llvm_dir.join("llvm");
    let cmake_configure_args: Vec<&str> = vec![
        "-S",
        llvm_src.to_str().unwrap(),
        "-B",
        build_dir.to_str().unwrap(),
        "-G",
        generator,
        "-DCMAKE_BUILD_TYPE=Release",
        "-DLLVM_ENABLE_PROJECTS=clang;lld;clang-tools-extra",
        "-DLLVM_ENABLE_RUNTIMES=compiler-rt",
        &install_prefix,
    ];

    let status = Command::new("cmake")
        .args(&cmake_configure_args)
        .status()
        .expect("failed to run cmake — is cmake installed?");
    if !status.success() {
        eprintln!("cmake configure failed");
        std::process::exit(1);
    }

    let num_cpus = std::thread::available_parallelism()
        .map(|n| n.get().to_string())
        .unwrap_or_else(|_| "4".to_string());

    println!("Building LLVM with {num_cpus} parallel jobs (this will take a while)...");
    let build_path = build_dir.to_str().unwrap().to_string();
    run_cmd(
        "cmake",
        &["--build", &build_path, "--config", "Release", "--parallel", &num_cpus],
        &llvm_dir,
        "LLVM build",
    );

    println!("Installing LLVM to {}...", install_dir.display());
    run_cmd(
        "cmake",
        &["--install", &build_path],
        &llvm_dir,
        "LLVM install",
    );

    configure_ce_for_upstream(&install_dir);

    println!("\n✅ LLVM upstream build complete!");
    println!("   Clang: {}/bin/clang++", install_dir.display());
}

fn configure_ce_for_upstream(install_dir: &PathBuf) {
    let ce_config_dir = project_root()
        .join("bin")
        .join("compiler-explorer")
        .join("etc")
        .join("config");

    if !ce_config_dir.exists() {
        println!(
            "⚠ Compiler Explorer not found — download it first, then re-run this command to configure."
        );
        return;
    }

    let clang_path = install_dir.join("bin").join("clang++");
    let config_file = ce_config_dir.join("c++.local.properties");

    let config = format!(
        "\
# Auto-generated by llvm-airfryer
compilers=&clang-upstream

group.clang-upstream.compilers=clang-upstream-main
group.clang-upstream.groupName=Clang Upstream (main)
group.clang-upstream.intelAsm=-mllvm --x86-asm-syntax=intel
group.clang-upstream.compilerType=clang
group.clang-upstream.compilerCategories=clang
group.clang-upstream.supportsBinary=true
group.clang-upstream.supportsBinaryObject=true
group.clang-upstream.supportsExecute=true

compiler.clang-upstream-main.exe={}
compiler.clang-upstream-main.name=Clang Upstream (main)
",
        clang_path.display()
    );

    std::fs::write(&config_file, &config).expect("failed to write CE config");
    println!(
        "📝 Compiler Explorer configured: {}",
        config_file.display()
    );
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
        1 => run_compiler_explorer(),
        2 => build_llvm_upstream(),
        3 => {
            println!("Goodbye!");
            std::process::exit(0);
        }
        _ => unreachable!(),
    }
}
