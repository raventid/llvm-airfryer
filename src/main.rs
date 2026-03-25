use dialoguer::console::style;
use dialoguer::{FuzzySelect, Input, Select, theme::ColorfulTheme};
use serde::{Deserialize, Serialize};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;

const COMPILER_EXPLORER_REPO: &str = "git@github.com:compiler-explorer/compiler-explorer.git";
const ZIG_REPO: &str = "git@github.com:ziglang/zig.git";
const CE_DEFAULT_PORT: u16 = 10240;

const MENU_ITEMS: &[&str] = &[
    "Download Compiler Explorer",
    "Run Compiler Explorer",
    "Build LLVM Upstream",
    "Build LLVM Branch",
    "Build Zig (Custom LLVM)",
    "CE Flag Presets",
    "Exit",
];

fn airfryer_home() -> PathBuf {
    match std::env::var("LLVM_AIRFRYER_HOME") {
        Ok(home) => PathBuf::from(shellexpand::tilde(&home).into_owned()),
        Err(_) => {
            eprintln!("{}: LLVM_AIRFRYER_HOME environment variable is not set.", style("Error").red().bold());
            eprintln!();
            eprintln!("This variable must point to the directory where llvm-airfryer stores");
            eprintln!("its builds, Compiler Explorer, and configuration.");
            eprintln!();
            eprintln!("Add this to your shell config (~/.zshrc or ~/.bashrc):");
            eprintln!();
            eprintln!("  {}",
                style("export LLVM_AIRFRYER_HOME=\"$HOME/.llvm_airfryer\"").green()
            );
            eprintln!();
            eprintln!("Then restart your shell or run: {}", style("source ~/.zshrc").green());
            std::process::exit(1);
        }
    }
}

fn builds_dir() -> PathBuf {
    airfryer_home().join("builds")
}

fn ce_dir() -> PathBuf {
    airfryer_home().join("compiler-explorer")
}

fn config_path() -> PathBuf {
    airfryer_home().join("config.toml")
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Config {
    #[serde(default)]
    llvm_source_path: Option<String>,
}

impl Config {
    fn load() -> Self {
        let path = config_path();
        if path.exists() {
            let contents = std::fs::read_to_string(&path).unwrap_or_default();
            toml::from_str(&contents).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    fn save(&self) {
        let home = airfryer_home();
        std::fs::create_dir_all(&home).expect("failed to create airfryer home directory");
        let contents = toml::to_string_pretty(self).expect("failed to serialize config");
        std::fs::write(config_path(), contents).expect("failed to write config.toml");
    }
}

fn download_compiler_explorer() {
    let dest = ce_dir();

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

    let home = airfryer_home();
    std::fs::create_dir_all(&home).expect("failed to create airfryer home directory");

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
    let ce_path = ce_dir();

    if !ce_path.exists() {
        eprintln!("Compiler Explorer not found. Please download it first.");
        std::process::exit(1);
    }

    let node_modules = ce_path.join("node_modules");
    if !node_modules.exists() {
        println!("Installing npm dependencies (first run)...");
        let status = Command::new("npm")
            .arg("install")
            .current_dir(&ce_path)
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
        .current_dir(&ce_path)
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

fn sanitize_branch_name(branch: &str) -> String {
    branch.replace('/', "-")
}

fn prompt_llvm_dir() -> Option<PathBuf> {
    let config = Config::load();
    let env_default = std::env::var("LLVM_AIRFRYER_LLVM_SOURCE_PATH")
        .ok()
        .or(config.llvm_source_path.clone());
    let theme = ColorfulTheme::default();

    let llvm_path: String = match env_default {
        Some(ref default_path) => Input::with_theme(&theme)
            .with_prompt("Path to llvm-project directory")
            .default(default_path.clone())
            .interact_text()
            .expect("failed to read input"),
        None => Input::with_theme(&theme)
            .with_prompt("Path to llvm-project directory")
            .interact_text()
            .expect("failed to read input"),
    };

    let trimmed = llvm_path.trim();
    if trimmed.is_empty() {
        return None;
    }

    let llvm_dir = PathBuf::from(shellexpand::tilde(trimmed).into_owned())
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

    // Persist the validated path to config
    let mut config = Config::load();
    config.llvm_source_path = Some(llvm_dir.display().to_string());
    config.save();

    Some(llvm_dir)
}

fn build_and_install_llvm(llvm_dir: &PathBuf, branch: &str, install_dir: &PathBuf) {
    println!("Switching to branch '{branch}'...");
    run_cmd("git", &["checkout", branch], llvm_dir, &format!("git checkout {branch}"));

    println!("Pulling latest changes...");
    // pull may fail for local-only branches — that's okay
    let _ = Command::new("git")
        .args(["pull"])
        .current_dir(llvm_dir)
        .status();

    let build_dir_name = format!("build-airfryer-{}", sanitize_branch_name(branch));
    let build_dir = llvm_dir.join(&build_dir_name);

    std::fs::create_dir_all(&build_dir).expect("failed to create build directory");
    std::fs::create_dir_all(install_dir).expect("failed to create install directory");

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
        llvm_dir,
        "LLVM build",
    );

    println!("Installing LLVM to {}...", install_dir.display());
    run_cmd(
        "cmake",
        &["--install", &build_path],
        llvm_dir,
        "LLVM install",
    );
}

fn build_llvm_upstream() -> bool {
    let Some(llvm_dir) = prompt_llvm_dir() else { return false };
    let install_dir = builds_dir().join("llvm-upstream");

    build_and_install_llvm(&llvm_dir, "main", &install_dir);
    regenerate_ce_config();

    println!("\n✅ LLVM upstream build complete!");
    println!("   Clang: {}/bin/clang++", install_dir.display());
    true
}

fn git_branches(repo_dir: &PathBuf) -> Vec<String> {
    let output = Command::new("git")
        .args(["branch", "--format=%(refname:short)", "--sort=-committerdate"])
        .current_dir(repo_dir)
        .output()
        .expect("failed to list git branches");

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.to_string())
        .collect()
}

fn build_llvm_branch() -> bool {
    let Some(llvm_dir) = prompt_llvm_dir() else { return false };

    let branches = git_branches(&llvm_dir);
    if branches.is_empty() {
        eprintln!("No branches found in {}", llvm_dir.display());
        std::process::exit(1);
    }

    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Branch to build (type to search)")
        .items(&branches)
        .default(0)
        .interact_opt()
        .expect("failed to select branch");

    let Some(idx) = selection else { return false };
    let branch = &branches[idx];

    let dir_name = format!("llvm-{}", sanitize_branch_name(branch));
    let install_dir = builds_dir().join(&dir_name);

    build_and_install_llvm(&llvm_dir, branch, &install_dir);
    regenerate_ce_config();

    println!("\n✅ LLVM branch '{branch}' build complete!");
    println!("   Clang: {}/bin/clang++", install_dir.display());
    true
}

fn build_zig_custom_llvm() -> bool {
    let bd = builds_dir();
    let zig_src = bd.join("zig-source");

    // Clone or update Zig source
    if zig_src.exists() {
        println!("Updating Zig source...");
        run_cmd("git", &["fetch", "--all"], &zig_src, "git fetch");
    } else {
        println!("Cloning Zig repository...");
        std::fs::create_dir_all(&bd).expect("failed to create builds directory");
        let status = Command::new("git")
            .args(["clone", ZIG_REPO])
            .arg(&zig_src)
            .status()
            .expect("failed to run git clone");
        if !status.success() {
            eprintln!("git clone failed for Zig");
            std::process::exit(1);
        }
    }

    // Pick Zig branch/tag
    let zig_tags = git_tags(&zig_src);
    let zig_branches = git_branches(&zig_src);
    let mut zig_refs: Vec<String> = Vec::new();
    zig_refs.push("master".to_string());
    for tag in zig_tags.iter().rev().take(10) {
        zig_refs.push(format!("tag: {tag}"));
    }
    for branch in &zig_branches {
        if branch != "master" && !zig_refs.contains(branch) {
            zig_refs.push(branch.clone());
        }
    }

    let zig_sel = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Zig version to build (type to search)")
        .items(&zig_refs)
        .default(0)
        .interact_opt()
        .expect("failed to select Zig version");

    let Some(zig_idx) = zig_sel else { return false };

    let zig_ref = zig_refs[zig_idx]
        .strip_prefix("tag: ")
        .unwrap_or(&zig_refs[zig_idx]);

    println!("Checking out Zig '{zig_ref}'...");
    run_cmd("git", &["checkout", zig_ref], &zig_src, &format!("git checkout {zig_ref}"));

    // Pick which LLVM build to use
    let llvm_builds = discover_llvm_builds();
    if llvm_builds.is_empty() {
        eprintln!("No LLVM builds found. Build LLVM first.");
        std::process::exit(1);
    }

    let llvm_names: Vec<&str> = llvm_builds.iter().map(|(_, _, d)| d.as_str()).collect();
    let llvm_sel = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Which LLVM build to use?")
        .items(&llvm_names)
        .default(0)
        .interact_opt()
        .expect("failed to select LLVM build");

    let Some(llvm_idx) = llvm_sel else { return false };

    let (llvm_id, llvm_exe, _) = &llvm_builds[llvm_idx];
    let llvm_install_dir = llvm_exe.parent().unwrap().parent().unwrap();

    let zig_ref_sanitized = sanitize_branch_name(zig_ref);
    let install_dir = bd.join(format!("zig-{zig_ref_sanitized}-{llvm_id}"));
    let build_dir = zig_src.join(format!("build-airfryer-{llvm_id}"));

    std::fs::create_dir_all(&build_dir).expect("failed to create Zig build directory");
    std::fs::create_dir_all(&install_dir).expect("failed to create Zig install directory");

    let generator = if has_ninja() { "Ninja" } else { "Unix Makefiles" };
    let prefix_path = format!("-DCMAKE_PREFIX_PATH={}", llvm_install_dir.display());
    let install_prefix = format!("-DCMAKE_INSTALL_PREFIX={}", install_dir.display());

    println!("Configuring Zig (LLVM: {})...", llvm_install_dir.display());
    let status = Command::new("cmake")
        .args([
            "-S",
            zig_src.to_str().unwrap(),
            "-B",
            build_dir.to_str().unwrap(),
            "-G",
            generator,
            "-DCMAKE_BUILD_TYPE=Release",
            &prefix_path,
            &install_prefix,
        ])
        .status()
        .expect("failed to run cmake for Zig");
    if !status.success() {
        eprintln!("Zig cmake configure failed");
        std::process::exit(1);
    }

    let num_cpus = std::thread::available_parallelism()
        .map(|n| n.get().to_string())
        .unwrap_or_else(|_| "4".to_string());

    println!("Building Zig with {num_cpus} parallel jobs...");
    let build_path = build_dir.to_str().unwrap().to_string();
    run_cmd(
        "cmake",
        &["--build", &build_path, "--config", "Release", "--parallel", &num_cpus],
        &zig_src,
        "Zig build",
    );

    println!("Installing Zig to {}...", install_dir.display());
    run_cmd(
        "cmake",
        &["--install", &build_path],
        &zig_src,
        "Zig install",
    );

    regenerate_ce_config();

    println!("\n✅ Zig build complete (backed by LLVM {llvm_id})!");
    println!("   Zig: {}/bin/zig", install_dir.display());
    true
}

fn git_tags(repo_dir: &PathBuf) -> Vec<String> {
    let output = Command::new("git")
        .args(["tag", "--sort=-version:refname"])
        .current_dir(repo_dir)
        .output()
        .expect("failed to list git tags");

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.to_string())
        .collect()
}

/// Discover all builds/llvm-* builds with clang++ binaries.
fn discover_llvm_builds() -> Vec<(String, PathBuf, String)> {
    let bd = builds_dir();
    let mut compilers: Vec<(String, PathBuf, String)> = Vec::new(); // (id, exe_path, display_name)

    if let Ok(entries) = std::fs::read_dir(&bd) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("llvm-") {
                continue;
            }
            let clang_bin = entry.path().join("bin").join("clang++");
            if !clang_bin.exists() {
                continue;
            }
            let branch_part = name.strip_prefix("llvm-").unwrap();
            let id = sanitize_branch_name(branch_part);
            let display = if branch_part == "upstream" {
                "Clang Upstream (main)".to_string()
            } else {
                format!("Clang ({branch_part})")
            };
            compilers.push((id, clang_bin, display));
        }
    }

    compilers.sort_by(|a, b| a.0.cmp(&b.0));
    compilers
}

/// Discover all builds/zig-* builds with zig binaries.
fn discover_zig_builds() -> Vec<(String, PathBuf, String)> {
    let bd = builds_dir();
    let mut compilers: Vec<(String, PathBuf, String)> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&bd) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("zig-") || name == "zig-source" {
                continue;
            }
            let zig_bin = entry.path().join("bin").join("zig");
            if !zig_bin.exists() {
                continue;
            }
            let label = name.strip_prefix("zig-").unwrap();
            let id = sanitize_branch_name(label);
            let display = format!("Zig ({label})");
            compilers.push((id, zig_bin, display));
        }
    }

    compilers.sort_by(|a, b| a.0.cmp(&b.0));
    compilers
}

/// Scan all builds/llvm-* and builds/zig-* directories and generate CE configs.
fn regenerate_ce_config() {
    let ce_config_dir = ce_dir()
        .join("etc")
        .join("config");

    if !ce_config_dir.exists() {
        println!(
            "⚠ Compiler Explorer not found — download it first, then re-run to configure."
        );
        return;
    }

    let clang_compilers = discover_llvm_builds();
    let zig_compilers = discover_zig_builds();

    if clang_compilers.is_empty() && zig_compilers.is_empty() {
        println!("⚠ No custom builds found in {}", builds_dir().display());
        return;
    }

    // --- C++ config ---
    if !clang_compilers.is_empty() {
        let group_refs: Vec<String> = clang_compilers.iter().map(|(id, _, _)| format!("&clang-{id}")).collect();
        let mut cpp_config = String::from("# Auto-generated by llvm-airfryer — do not edit manually\n");
        cpp_config.push_str(&format!(
            "compilers=&gcc:&clang:{}\n",
            group_refs.join(":")
        ));

        for (id, exe, display) in &clang_compilers {
            let group_id = format!("clang-{id}");
            let compiler_id = format!("clang-{id}-compiler");
            cpp_config.push_str(&format!(
                "\n\
                 group.{group_id}.compilers={compiler_id}\n\
                 group.{group_id}.groupName={display}\n\
                 group.{group_id}.intelAsm=-mllvm --x86-asm-syntax=intel\n\
                 group.{group_id}.compilerType=clang\n\
                 group.{group_id}.compilerCategories=clang\n\
                 group.{group_id}.supportsBinary=true\n\
                 group.{group_id}.supportsBinaryObject=true\n\
                 group.{group_id}.supportsExecute=true\n\
                 \n\
                 compiler.{compiler_id}.exe={}\n\
                 compiler.{compiler_id}.name={display}\n",
                exe.display()
            ));
        }

        std::fs::write(ce_config_dir.join("c++.local.properties"), &cpp_config)
            .expect("failed to write C++ CE config");
    }

    // --- LLVM IR config ---
    if !clang_compilers.is_empty() {
        let ir_compiler_ids: Vec<String> = clang_compilers
            .iter()
            .map(|(id, _, _)| format!("ir-{id}"))
            .collect();
        let mut ir_config = String::from("# Auto-generated by llvm-airfryer — do not edit manually\n");
        ir_config.push_str(&format!(
            "compilers=irclang:llc:opt:{}\n",
            ir_compiler_ids.join(":")
        ));

        for (id, exe, display) in &clang_compilers {
            let compiler_id = format!("ir-{id}");
            let ir_display = display.replace("Clang", "Clang IR");
            ir_config.push_str(&format!(
                "\ncompiler.{compiler_id}.exe={}\n\
                 compiler.{compiler_id}.name={ir_display}\n\
                 compiler.{compiler_id}.intelAsm=-masm=intel\n\
                 compiler.{compiler_id}.options=-x ir\n\
                 compiler.{compiler_id}.compilerType=clang\n\
                 compiler.{compiler_id}.supportsBinary=true\n\
                 compiler.{compiler_id}.supportsExecute=true\n",
                exe.display()
            ));
        }

        std::fs::write(ce_config_dir.join("llvm.local.properties"), &ir_config)
            .expect("failed to write LLVM IR CE config");
    }

    // --- Zig config ---
    if !zig_compilers.is_empty() {
        let zig_ids: Vec<String> = zig_compilers
            .iter()
            .map(|(id, _, _)| format!("zig-{id}"))
            .collect();
        let mut zig_config = String::from("# Auto-generated by llvm-airfryer — do not edit manually\n");
        zig_config.push_str(&format!(
            "compilers=zig:{}\n",
            zig_ids.join(":")
        ));

        for (id, exe, display) in &zig_compilers {
            let compiler_id = format!("zig-{id}");
            zig_config.push_str(&format!(
                "\ncompiler.{compiler_id}.exe={}\n\
                 compiler.{compiler_id}.name={display}\n\
                 compiler.{compiler_id}.compilerType=zig\n\
                 compiler.{compiler_id}.supportsBinary=true\n\
                 compiler.{compiler_id}.supportsExecute=true\n\
                 compiler.{compiler_id}.versionFlag=version\n\
                 compiler.{compiler_id}.isSemVer=true\n",
                exe.display()
            ));
        }

        std::fs::write(ce_config_dir.join("zig.local.properties"), &zig_config)
            .expect("failed to write Zig CE config");
    }

    let total = clang_compilers.len() + zig_compilers.len();
    println!("📝 Compiler Explorer configured with {total} custom compiler(s):");
    for (_, _, display) in &clang_compilers {
        println!("   • {display} (C++ & LLVM IR)");
    }
    for (_, _, display) in &zig_compilers {
        println!("   • {display}");
    }
}

const FLAG_PRESETS: &[(&str, &str)] = &[
    ("AVX-512 (x86_64)", "--target=x86_64-unknown-linux-gnu -O3 -S -mavx512f -mavx512vl -fno-exceptions -fno-rtti"),
    ("AVX2 (x86_64)", "--target=x86_64-unknown-linux-gnu -O3 -S -mavx2 -fno-exceptions -fno-rtti"),
    ("AArch64 NEON", "--target=aarch64-unknown-linux-gnu -O3 -S -fno-exceptions -fno-rtti"),
    ("AArch64 SVE", "--target=aarch64-unknown-linux-gnu -O3 -S -march=armv8-a+sve -fno-exceptions -fno-rtti"),
    ("AArch64 SVE2", "--target=aarch64-unknown-linux-gnu -O3 -S -march=armv9-a+sve2 -fno-exceptions -fno-rtti"),
    ("RISC-V Vector (RVV)", "--target=riscv64-unknown-linux-gnu -O3 -S -march=rv64gcv -fno-exceptions -fno-rtti"),
    ("x86_64 baseline", "--target=x86_64-unknown-linux-gnu -O3 -S -fno-exceptions -fno-rtti"),
];

fn print_colored_flags(flags: &str) {
    let colored_parts: Vec<String> = flags.split_whitespace().map(|flag| {
        if flag.starts_with("--") {
            if let Some(eq) = flag.find('=') {
                format!("{}{}", style(&flag[..=eq]).blue(), style(&flag[eq + 1..]).green())
            } else {
                format!("{}", style(flag).blue())
            }
        } else {
            format!("{}", style(flag).magenta())
        }
    }).collect();
    println!("{}", colored_parts.join("  "));
}

fn show_flag_presets() -> bool {
    let preset_names: Vec<&str> = FLAG_PRESETS.iter().map(|(name, _)| *name).collect();

    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select a flag preset")
        .items(&preset_names)
        .default(0)
        .interact_opt()
        .expect("failed to render preset menu");

    let Some(idx) = selection else {
        return false;
    };

    let (name, flags) = FLAG_PRESETS[idx];
    println!("\n{}:\n", style(name).cyan().bold());
    print_colored_flags(flags);
    println!("\n{}", style("(copy the line above and paste into the Compiler Explorer options field)").dim());
    println!();
    true
}

fn main() {
    let home = airfryer_home();
    println!("🔥 LLVM Airfryer — LLVM compiler framework development toolkit");
    println!("   {}: {}\n", style("Home").dim(), style(home.display()).dim());

    loop {
        let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt("What would you like to do? (type to search, Esc to quit)")
            .items(MENU_ITEMS)
            .default(0)
            .interact_opt()
            .expect("failed to render interactive menu");

        let Some(idx) = selection else {
            println!("Goodbye!");
            break;
        };

        let go_back = match idx {
            0 => { download_compiler_explorer(); false }
            1 => { run_compiler_explorer(); false }
            2 => !build_llvm_upstream(),
            3 => !build_llvm_branch(),
            4 => !build_zig_custom_llvm(),
            5 => !show_flag_presets(),
            6 => {
                println!("Goodbye!");
                break;
            }
            _ => unreachable!(),
        };

        if !go_back {
            break;
        }
        // go_back == true means user pressed Esc in a sub-prompt, loop back to menu
        println!();
    }
}
