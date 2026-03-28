use dialoguer::console::{Term, style};
use dialoguer::{FuzzySelect, Input, Select, theme::ColorfulTheme};
use serde::{Deserialize, Serialize};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;

mod resources;

const COMPILER_EXPLORER_REPO: &str = "git@github.com:compiler-explorer/compiler-explorer.git";
const ZIG_REPO: &str = "git@github.com:ziglang/zig.git";
const CE_DEFAULT_PORT: u16 = 10240;

const MENU_ITEMS: &[&str] = &[
    "Download Compiler Explorer",
    "Run Compiler Explorer",
    "Stop Compiler Explorer",
    "Build LLVM Upstream",
    "Build LLVM Branch",
    "Build Zig (Custom LLVM)",
    "CE Flag Presets",
    "Interesting Resources",
    "Help & Configuration",
    "Exit",
];

fn airfryer_home() -> PathBuf {
    match std::env::var("LLVM_AIRFRYER_HOME") {
        Ok(home) => PathBuf::from(shellexpand::tilde(&home).into_owned()),
        Err(_) => {
            // Check if default location exists (previously set up)
            let default = default_home();
            if default.join("config.toml").exists() {
                return default;
            }
            // First run — launch setup wizard
            run_setup_wizard()
        }
    }
}

fn default_home() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME environment variable not set");
    PathBuf::from(home).join(".llvm_airfryer")
}

fn run_setup_wizard() -> PathBuf {
    let _ = Term::stdout().clear_screen();
    println!("{}", style("═══ LLVM Airfryer — First-Time Setup ═══").cyan().bold());
    println!();
    println!("Welcome! Let's configure where llvm-airfryer stores its data.");
    println!();

    let theme = ColorfulTheme::default();
    let default_path = default_home();

    // 1. Home directory
    println!("{}", style("Step 1: Home directory").yellow().bold());
    println!("  This is where config, builds, and Compiler Explorer will live.");
    println!("  Heavy build artifacts can be moved later with env variables.\n");

    let home_input: String = Input::with_theme(&theme)
        .with_prompt("Home directory")
        .default(default_path.display().to_string())
        .interact_text()
        .expect("failed to read input");

    let home = PathBuf::from(shellexpand::tilde(home_input.trim()).into_owned());
    std::fs::create_dir_all(&home).expect("failed to create home directory");

    // 2. LLVM source path (optional)
    println!("\n{}", style("Step 2: LLVM source path (optional)").yellow().bold());
    println!("  If you have a local clone of llvm-project, enter its path.");
    println!("  This is saved in {} and remembered for future sessions.", style("config.toml").green());
    println!("  You can also override it with the {} env variable.", style("LLVM_AIRFRYER_LLVM_SOURCE_PATH").green());
    println!("  If you skip this, the path will be saved to config automatically");
    println!("  the first time you run a build command.\n");

    let llvm_input: String = Input::with_theme(&theme)
        .with_prompt("Path to llvm-project (leave empty to skip)")
        .default(String::new())
        .show_default(false)
        .allow_empty(true)
        .interact_text()
        .expect("failed to read input");

    let llvm_source_path = if llvm_input.trim().is_empty() {
        None
    } else {
        let expanded = PathBuf::from(shellexpand::tilde(llvm_input.trim()).into_owned());
        match expanded.canonicalize() {
            Ok(p) if p.join("llvm").exists() => Some(p.display().to_string()),
            Ok(p) => {
                println!("  {} No 'llvm/' subdirectory found in {}. Skipping.", style("⚠").yellow(), p.display());
                None
            }
            Err(_) => {
                println!("  {} Path does not exist. Skipping.", style("⚠").yellow());
                None
            }
        }
    };

    // 3. Write config
    let config = Config { llvm_source_path };
    let config_contents = toml::to_string_pretty(&config).expect("failed to serialize config");
    std::fs::write(home.join("config.toml"), config_contents).expect("failed to write config.toml");

    // 4. Set env var for current session
    // Safe here: single-threaded at startup, no other threads reading env
    unsafe { std::env::set_var("LLVM_AIRFRYER_HOME", &home); }

    // 5. Create env file and bin directory
    std::fs::create_dir_all(home.join("bin")).expect("failed to create bin directory");
    write_env_file(&home);

    let source_line = format!(". \"{}\"", home.join("env").display());
    let mut auto_updated = false;

    // 6. Detect shell and offer to update config automatically
    println!("\n{}", style("Step 3: Shell configuration").yellow().bold());
    println!("  llvm-airfryer needs your shell to load an {} file on startup.", style("env").green());
    println!("  This adds the binary to your {}.", style("PATH").green());
    println!();

    if let Some((shell_name, config_path)) = detect_shell_config() {
        println!("  Detected your shell: {}", style(&shell_name).cyan().bold());
        println!("  We can add this line to {} automatically:", style(&config_path).bold());
        println!("  {}", style(&source_line).green().bold());
        println!();

        let choice = Select::with_theme(&theme)
            .with_prompt(format!("Add to {}?", config_path))
            .items(&["Yes, update my shell config", "No, I'll do it manually"])
            .default(0)
            .interact()
            .unwrap_or(1);

        if choice == 0 {
            let path = PathBuf::from(shellexpand::tilde(&config_path).into_owned());
            let mut contents = std::fs::read_to_string(&path).unwrap_or_default();
            if !contents.contains("llvm_airfryer/env") {
                if !contents.is_empty() && !contents.ends_with('\n') {
                    contents.push('\n');
                }
                contents.push_str(&format!("\n# llvm-airfryer\n{}\n", source_line));
                std::fs::write(&path, contents).expect("failed to update shell config");
                println!("\n  {} Updated {}", style("✔").green().bold(), style(&config_path).bold());
                auto_updated = true;
            } else {
                println!("\n  {} Already configured in {}", style("✔").green(), config_path);
                auto_updated = true;
            }
        }
    }

    // 7. Show completion message
    println!("\n{}", style("═══ Setup Complete! ═══").green().bold());
    println!();
    println!("  Home directory: {}", style(home.display()).bold());
    if let Some(ref llvm) = config.llvm_source_path {
        println!("  LLVM source:    {}", style(llvm).bold());
    }
    if auto_updated {
        println!("  Shell config:   {} updated automatically", style("✔").green());
    }
    println!();

    if !auto_updated {
        // Show the framed manual instructions
        // Line styles: 'H' = heading (yellow bold), 'G' = green bold (command),
        // 'D' = dim, ' ' = plain, 'E' = empty
        let w: usize = 64;
        let top    = format!("╔{}╗", "═".repeat(w));
        let bottom = format!("╚{}╝", "═".repeat(w));
        let empty_line = format!("║{}║", " ".repeat(w));

        let source_padded = format!("    {}", source_line);
        let reload_line = format!("    {}", source_line);
        let lines: Vec<(&str, char)> = vec![
            ("", 'E'),
            ("  ACTION REQUIRED:", 'H'),
            ("", 'E'),
            ("  Add this line to your shell config:", ' '),
            ("", 'E'),
            (&source_padded, 'G'),
            ("", 'E'),
            ("  This adds the llvm-airfryer binary to your PATH.", 'D'),
            ("", 'E'),
            ("  Depending on your shell, your config file might differ:", ' '),
            ("    zsh  — ~/.zshrc", 'D'),
            ("    bash — ~/.bashrc or ~/.bash_profile", 'D'),
            ("    fish — ~/.config/fish/config.fish", 'D'),
            ("", 'E'),
            ("  Restart the shell or type this command again to apply:", ' '),
            ("", 'E'),
            (&reload_line, 'G'),
            ("", 'E'),
        ];

        println!("  {}", style(&top).yellow());
        for (text, kind) in &lines {
            if *kind == 'E' {
                println!("  {}", style(&empty_line).yellow());
            } else {
                let padded = format!("{:<width$}", text, width = w);
                let styled_content = match kind {
                    'H' => style(padded).yellow().bold().to_string(),
                    'G' => style(padded).green().bold().to_string(),
                    'D' => style(padded).dim().to_string(),
                    _   => padded,
                };
                println!("  {}{}{}",
                    style("║").yellow(),
                    styled_content,
                    style("║").yellow());
            }
        }
        println!("  {}", style(&bottom).yellow());
        println!();
    }

    if auto_updated {
        println!("Restart your shell and run {} to get started.",
            style("llvm-airfryer").bold());
        println!();
        println!("Or to activate right now, paste this into your current shell:");
        println!("  {}", style(&source_line).green().bold());
    } else {
        println!("Then run {} to get started.", style("llvm-airfryer").bold());
    }
    println!();

    write_install_marker(&home);
    std::process::exit(0);
}

/// Detect the user's shell and return (shell_name, config_file_path).
fn detect_shell_config() -> Option<(String, String)> {
    let shell = std::env::var("SHELL").ok()?;
    if shell.ends_with("/zsh") {
        Some(("zsh".into(), "~/.zshrc".into()))
    } else if shell.ends_with("/bash") {
        let home = std::env::var("HOME").ok()?;
        // Prefer .bashrc on Linux, .bash_profile on macOS
        let profile = if PathBuf::from(&home).join(".bash_profile").exists() {
            "~/.bash_profile"
        } else {
            "~/.bashrc"
        };
        Some(("bash".into(), profile.into()))
    } else if shell.ends_with("/fish") {
        Some(("fish".into(), "~/.config/fish/config.fish".into()))
    } else {
        None
    }
}

/// Write the chosen home path to a temp file so install.sh can find it.
/// Harmless in interactive use — the file is ignored if no one reads it.
fn write_install_marker(home: &PathBuf) {
    let marker = std::env::temp_dir().join("llvm_airfryer_install_home");
    let _ = std::fs::write(marker, home.display().to_string());
}

/// Write the env file that adds bin/ to PATH.
fn write_env_file(home: &PathBuf) {
    let env_file = home.join("env");
    let contents = format!(
        r#"#!/bin/sh
# llvm-airfryer shell setup — source this file in your shell config
# e.g.  . "{home}/env"

case ":${{PATH}}:" in
    *:"{home}/bin":*)
        ;;
    *)
        export PATH="{home}/bin:$PATH"
        ;;
esac
"#,
        home = home.display()
    );
    std::fs::write(&env_file, contents).expect("failed to write env file");
}

fn builds_dir() -> PathBuf {
    if let Ok(path) = std::env::var("LLVM_AIRFRYER_BUILDS_PATH") {
        return PathBuf::from(shellexpand::tilde(&path).into_owned());
    }
    airfryer_home().join("builds")
}

fn ce_dir() -> PathBuf {
    if let Ok(path) = std::env::var("LLVM_AIRFRYER_CE_PATH") {
        return PathBuf::from(shellexpand::tilde(&path).into_owned());
    }
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

fn download_compiler_explorer() -> bool {
    let dest = ce_dir();

    if dest.exists() {
        println!("{} Compiler Explorer already exists at {}", style("ℹ").cyan(), style(dest.display()).dim());
        println!("  Pulling latest changes from {}...", style(COMPILER_EXPLORER_REPO).blue().underlined());
        let status = Command::new("git")
            .args(["pull"])
            .current_dir(&dest)
            .status()
            .expect("failed to run git pull");
        if !status.success() {
            eprintln!("{} git pull failed", style("✗").red().bold());
            return false;
        }
        println!("\n{} Compiler Explorer updated successfully.", style("✔").green().bold());
        return true;
    }

    let home = airfryer_home();
    std::fs::create_dir_all(&home).expect("failed to create airfryer home directory");

    println!("{} Downloading Compiler Explorer from:", style("⬇").cyan().bold());
    println!("  {}", style(COMPILER_EXPLORER_REPO).blue().underlined());
    println!("  → {}\n", style(dest.display()).dim());
    let status = Command::new("git")
        .args(["clone", "--depth=1", COMPILER_EXPLORER_REPO])
        .arg(&dest)
        .status()
        .expect("failed to run git clone");

    if !status.success() {
        eprintln!("\n{} git clone failed", style("✗").red().bold());
        return false;
    }
    println!("\n{} Compiler Explorer downloaded successfully!", style("✔").green().bold());
    println!("  Applying LLVM Airfryer branding...");
    patch_compiler_explorer();
    println!("  You can now run it from the main menu.");
    true
}

fn patch_compiler_explorer() {
    let ce_path = ce_dir();

    // 1. Create custom logo overlay SVG
    let logo_svg = r##"<svg viewBox="0 0 165 50" xmlns="http://www.w3.org/2000/svg">
  <g transform="translate(50,48)rotate(-25)" font-weight="bold" font-family="sans-serif" font-size="14">
    <text textLength="120" lengthAdjust="spacingAndGlyphs">AIRFRYER</text>
    <text textLength="120" lengthAdjust="spacingAndGlyphs" fill="#e85d04" dx="-1" dy="-1">AIRFRYER</text>
  </g>
</svg>"##;

    let public_dir = ce_path.join("public");
    std::fs::write(public_dir.join("site-logo-airfryer.svg"), logo_svg)
        .expect("failed to write airfryer logo SVG");

    // 2. Patch logo.pug to add airfryer branch
    let logo_pug_path = ce_path.join("views").join("logo.pug");
    if let Ok(content) = std::fs::read_to_string(&logo_pug_path) {
        if !content.contains("airfryer") {
            let patched = content.replace(
                r#"else if extraBodyClass === "dev""#,
                "else if extraBodyClass === \"airfryer\"\n    img(src=staticRoot + \"site-logo-airfryer.svg\" alt=\"LLVM Airfryer\" height=\"50\" width=\"165\" style=\"position: absolute; top: 0; right: 0;\")\n  else if extraBodyClass === \"dev\"",
            );
            std::fs::write(&logo_pug_path, patched)
                .expect("failed to patch logo.pug");
        }
    }

    // 3. Set extraBodyClass=airfryer in local config
    let config_dir = ce_path.join("etc").join("config");
    std::fs::create_dir_all(&config_dir).ok();
    let local_props = config_dir.join("compiler-explorer.local.properties");
    let mut props = if local_props.exists() {
        std::fs::read_to_string(&local_props).unwrap_or_default()
    } else {
        String::new()
    };
    if !props.contains("extraBodyClass") {
        if !props.is_empty() && !props.ends_with('\n') {
            props.push('\n');
        }
        props.push_str("extraBodyClass=airfryer\n");
        std::fs::write(&local_props, props)
            .expect("failed to write compiler-explorer.local.properties");
    }

    println!("  {} Branding applied.", style("✔").green());
}

fn find_available_port(start: u16) -> Option<u16> {
    (start..=u16::MAX).find(|&port| TcpListener::bind(("127.0.0.1", port)).is_ok())
}

fn run_compiler_explorer() -> bool {
    let home = airfryer_home();
    let ce_path = ce_dir();

    if !ce_path.exists() {
        eprintln!("Compiler Explorer not found. Please download it first.");
        return false;
    }

    // Check if CE is already running
    let pid_file = home.join("ce.pid");
    let port_file = home.join("ce.port");
    if pid_file.exists() {
        if let Ok(pid_str) = std::fs::read_to_string(&pid_file) {
            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                let alive = Command::new("kill")
                    .args(["-0", &pid.to_string()])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .is_ok_and(|s| s.success());
                if alive {
                    let port = port_file.exists()
                        .then(|| std::fs::read_to_string(&port_file).ok())
                        .flatten()
                        .and_then(|s| s.trim().parse::<u16>().ok())
                        .unwrap_or(CE_DEFAULT_PORT);
                    let url = format!("http://localhost:{port}");
                    println!("\n  {} Compiler Explorer is running on {} (PID {})",
                        style("ℹ").cyan(),
                        style(&url).cyan().underlined(),
                        style(pid).bold());

                    let open = Select::with_theme(&ColorfulTheme::default())
                        .with_prompt("Open in browser?")
                        .items(&["Yes", "No"])
                        .default(0)
                        .interact()
                        .unwrap_or(1);
                    if open == 0 {
                        let _ = Command::new("open").arg(&url).status();
                    }
                    return true;
                }
            }
        }
        let _ = std::fs::remove_file(&pid_file);
        let _ = std::fs::remove_file(&port_file);
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
            return false;
        }
    }

    let port = match find_available_port(CE_DEFAULT_PORT) {
        Some(p) => p,
        None => {
            eprintln!("No available port found starting from {CE_DEFAULT_PORT}");
            return false;
        }
    };

    println!("Building and starting Compiler Explorer on port {}...", style(port).bold());

    let log_file_path = home.join("ce.log");
    let log_file = std::fs::File::create(&log_file_path)
        .expect("failed to create ce.log");
    let log_stderr = log_file.try_clone().expect("failed to clone log file handle");

    let child = Command::new("make")
        .arg("run")
        .env("EXTRA_ARGS", format!("--port {port}"))
        .current_dir(&ce_path)
        .stdout(log_file)
        .stderr(log_stderr)
        .spawn();

    let child = match child {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to start Compiler Explorer: {e}");
            return false;
        }
    };

    let pid = child.id();
    std::fs::write(&pid_file, pid.to_string()).expect("failed to write ce.pid");
    std::fs::write(&port_file, port.to_string()).expect("failed to write ce.port");

    // Tail the log file waiting for "Listening on" or an error
    use std::io::{BufRead, BufReader};
    let timeout = std::time::Duration::from_secs(120);
    let start = std::time::Instant::now();
    let poll_interval = std::time::Duration::from_millis(300);

    let mut reader = BufReader::new(
        std::fs::File::open(&log_file_path).expect("failed to open ce.log for reading")
    );

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => {
                // No new data yet — check timeout and whether process is still alive
                if start.elapsed() > timeout {
                    eprintln!("Timed out waiting for Compiler Explorer to start.");
                    eprintln!("Check logs at: {}", log_file_path.display());
                    return false;
                }
                // Check process is still running
                let alive = Command::new("kill")
                    .args(["-0", &pid.to_string()])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .is_ok_and(|s| s.success());
                if !alive {
                    eprintln!("Compiler Explorer process exited unexpectedly.");
                    eprintln!("Check logs at: {}", log_file_path.display());
                    let _ = std::fs::remove_file(&pid_file);
                    return false;
                }
                std::thread::sleep(poll_interval);
            }
            Ok(_) => {
                // Show startup logs so user can see progress
                print!("  {}", style(&line).dim());
                if line.contains("Listening on") {
                    println!();
                    println!("  {} Compiler Explorer started on {} (PID {})",
                        style("✔").green().bold(),
                        style(format!("http://localhost:{port}")).cyan().underlined(),
                        style(pid).bold());
                    println!("  Logs: {}", style(log_file_path.display()).dim());
                    return true;
                }
            }
            Err(e) => {
                eprintln!("Error reading log: {e}");
                return false;
            }
        }
    }
}

fn run_cmd(cmd: &str, args: &[&str], dir: &PathBuf, context: &str) -> bool {
    let status = match Command::new(cmd).args(args).current_dir(dir).status() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to run {context}: {e}");
            return false;
        }
    };
    if !status.success() {
        eprintln!("{context} failed");
        return false;
    }
    true
}

fn stop_compiler_explorer() -> bool {
    let home = airfryer_home();
    let pid_file = home.join("ce.pid");

    let pid_str = match std::fs::read_to_string(&pid_file) {
        Ok(s) => s,
        Err(_) => {
            println!("{} Compiler Explorer is not running (no PID file found).",
                style("ℹ").cyan());
            return true;
        }
    };

    let pid: i32 = match pid_str.trim().parse() {
        Ok(p) => p,
        Err(_) => {
            eprintln!("Invalid PID in ce.pid, removing stale file.");
            let _ = std::fs::remove_file(&pid_file);
            return false;
        }
    };

    // Check if process is alive
    let check = Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    if !check.is_ok_and(|s| s.success()) {
        println!("{} Compiler Explorer is not running (stale PID {}).",
            style("ℹ").cyan(), pid);
        let _ = std::fs::remove_file(&pid_file);
        let _ = std::fs::remove_file(&home.join("ce.port"));
        return true;
    }

    // Send SIGTERM to the process directly
    println!("Stopping Compiler Explorer (PID {})...", style(pid).bold());
    let _ = Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    // Also try to kill the process group (may fail silently if not a group leader)
    let _ = Command::new("kill")
        .args(["-TERM", &format!("-{}", pid)])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    let _ = std::fs::remove_file(&pid_file);
    let _ = std::fs::remove_file(&home.join("ce.port"));
    println!("{} Compiler Explorer stopped.", style("✔").green().bold());
    true
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

    let llvm_dir = match PathBuf::from(shellexpand::tilde(trimmed).into_owned()).canonicalize() {
        Ok(p) => p,
        Err(_) => {
            eprintln!("Path does not exist: {llvm_path}");
            return None;
        }
    };

    if !llvm_dir.join("llvm").exists() {
        eprintln!(
            "Invalid llvm-project directory: expected 'llvm/' subdirectory in {}",
            llvm_dir.display()
        );
        return None;
    }

    // Only save to config if no path is set yet (don't overwrite one-off usage)
    let config = Config::load();
    if config.llvm_source_path.is_none() {
        let mut config = config;
        config.llvm_source_path = Some(llvm_dir.display().to_string());
        config.save();
    }

    Some(llvm_dir)
}

fn build_and_install_llvm(llvm_dir: &PathBuf, branch: &str, install_dir: &PathBuf) -> bool {
    println!("Switching to branch '{branch}'...");
    if !run_cmd("git", &["checkout", branch], llvm_dir, &format!("git checkout {branch}")) {
        return false;
    }

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
        return false;
    }

    let num_cpus = std::thread::available_parallelism()
        .map(|n| n.get().to_string())
        .unwrap_or_else(|_| "4".to_string());

    println!("Building LLVM with {num_cpus} parallel jobs (this will take a while)...");
    let build_path = build_dir.to_str().unwrap().to_string();
    if !run_cmd(
        "cmake",
        &["--build", &build_path, "--config", "Release", "--parallel", &num_cpus],
        llvm_dir,
        "LLVM build",
    ) {
        return false;
    }

    println!("Installing LLVM to {}...", install_dir.display());
    if !run_cmd(
        "cmake",
        &["--install", &build_path],
        llvm_dir,
        "LLVM install",
    ) {
        return false;
    }
    true
}

fn build_llvm_upstream() -> bool {
    let Some(llvm_dir) = prompt_llvm_dir() else { return false };
    let install_dir = builds_dir().join("llvm-upstream");

    if !build_and_install_llvm(&llvm_dir, "main", &install_dir) {
        return false;
    }
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
        return false;
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

    if !build_and_install_llvm(&llvm_dir, branch, &install_dir) {
        return false;
    }
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
        if !run_cmd("git", &["fetch", "--all"], &zig_src, "git fetch") {
            return false;
        }
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
            return false;
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
    if !run_cmd("git", &["checkout", zig_ref], &zig_src, &format!("git checkout {zig_ref}")) {
        return false;
    }

    // Pick which LLVM build to use
    let llvm_builds = discover_llvm_builds();
    if llvm_builds.is_empty() {
        eprintln!("No LLVM builds found. Build LLVM first.");
        return false;
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

    // Detect LLVM version and check compatibility with Zig
    let llvm_config = llvm_install_dir.join("bin").join("llvm-config");
    if llvm_config.exists() {
        if let Ok(output) = Command::new(&llvm_config).arg("--version").output() {
            let llvm_version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let llvm_major = llvm_version.split('.').next().unwrap_or("?");

            let findllvm_path = zig_src.join("cmake").join("Findllvm.cmake");
            if findllvm_path.exists() {
                if let Ok(findllvm) = std::fs::read_to_string(&findllvm_path) {
                    // Look for the expected version pattern, e.g. "expected LLVM 21"
                    let expected_major = findllvm.lines()
                        .find(|l| l.contains("expected LLVM"))
                        .and_then(|l| {
                            l.split("expected LLVM ")
                                .nth(1)
                                .and_then(|s| s.split(|c: char| !c.is_ascii_digit()).next())
                        });

                    if let Some(expected) = expected_major {
                        if expected != llvm_major {
                            let version_gap: i32 = llvm_major.parse::<i32>().unwrap_or(0)
                                - expected.parse::<i32>().unwrap_or(0);
                            let big_gap = version_gap.abs() > 1;

                            println!("\n  {} Zig expects LLVM {} but your build is LLVM {}",
                                style("⚠").yellow().bold(), style(expected).bold(), style(&llvm_version).bold());

                            if big_gap {
                                println!("    {} LLVM APIs change across major versions — build will likely fail",
                                    style("Major version gap!").red().bold());
                                println!("    even with the cmake patch due to C++ API incompatibilities.");
                                println!("    Consider building LLVM {} for Zig, or wait for Zig to support LLVM {}.\n",
                                    expected, llvm_major);
                            } else {
                                println!("    Minor version gap — patching cmake will likely work.\n");
                            }

                            let choice = Select::with_theme(&ColorfulTheme::default())
                                .with_prompt("How to proceed?")
                                .items(&[
                                    "Patch Zig cmake to skip version check",
                                    "Continue anyway (will likely fail)",
                                    "Abort",
                                ])
                                .default(if big_gap { 2 } else { 0 })
                                .interact_opt()
                                .expect("failed to render choice");

                            match choice {
                                Some(0) => {
                                    // Patch Findllvm.cmake to skip the version range check.
                                    // The check is a VERSION_LESS/VERSION_GREATER if-block
                                    // that adds to an ignore list and continues the loop.
                                    let mut skip_until_endif = false;
                                    let patched = findllvm.lines().map(|line| {
                                        if line.contains("VERSION_LESS") && line.contains("VERSION_GREATER") {
                                            skip_until_endif = true;
                                            format!("    # Patched by llvm-airfryer to accept any LLVM version")
                                        } else if skip_until_endif {
                                            if line.trim_start().starts_with("endif()") {
                                                skip_until_endif = false;
                                            }
                                            format!("    # {}", line.trim())
                                        } else {
                                            line.to_string()
                                        }
                                    }).collect::<Vec<_>>().join("\n");
                                    std::fs::write(&findllvm_path, patched)
                                        .expect("failed to patch Findllvm.cmake");
                                    println!("  {} Patched Zig cmake to accept LLVM {}",
                                        style("✔").green().bold(), &llvm_version);
                                }
                                Some(1) => {
                                    println!("  Continuing without patching...");
                                }
                                _ => return false,
                            }
                        }
                    }
                }
            }
        }
    }

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
        return false;
    }

    let num_cpus = std::thread::available_parallelism()
        .map(|n| n.get().to_string())
        .unwrap_or_else(|_| "4".to_string());

    println!("Building Zig with {num_cpus} parallel jobs...");
    let build_path = build_dir.to_str().unwrap().to_string();
    if !run_cmd(
        "cmake",
        &["--build", &build_path, "--config", "Release", "--parallel", &num_cpus],
        &zig_src,
        "Zig build",
    ) {
        return false;
    }

    println!("Installing Zig to {}...", install_dir.display());
    if !run_cmd(
        "cmake",
        &["--install", &build_path],
        &zig_src,
        "Zig install",
    ) {
        return false;
    }

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

fn show_help() {
    let home = airfryer_home();
    let bd = builds_dir();
    let ce = ce_dir();

    println!();
    println!("{}", style("═══ LLVM Airfryer — Help & Configuration ═══").cyan().bold());

    println!("\n{}", style("ABOUT").yellow().bold());
    println!("  LLVM Airfryer is an interactive toolkit for building LLVM, Zig,");
    println!("  and running Compiler Explorer with custom compiler builds.");

    println!("\n{}", style("DIRECTORY LAYOUT").yellow().bold());
    println!("  {}   {}", style("Home:").bold(), home.display());
    println!("  {} {}", style("Builds:").bold(), bd.display());
    println!("  {}     {}", style("CE:").bold(), ce.display());
    println!("  {} {}", style("Config:").bold(), config_path().display());

    println!("\n{}", style("ENVIRONMENT VARIABLES").yellow().bold());
    println!("  {}  {}", style("LLVM_AIRFRYER_HOME").green(), style("(required)").dim());
    println!("    Root directory for all airfryer data.");
    println!("    Example: export LLVM_AIRFRYER_HOME=\"$HOME/.llvm_airfryer\"");

    println!("\n  {}  {}", style("LLVM_AIRFRYER_BUILDS_PATH").green(), style("(optional)").dim());
    println!("    Override where LLVM/Zig builds are stored.");
    println!("    Useful for placing heavy builds on a separate disk.");
    println!("    Default: $LLVM_AIRFRYER_HOME/builds");

    println!("\n  {}  {}", style("LLVM_AIRFRYER_CE_PATH").green(), style("(optional)").dim());
    println!("    Override where Compiler Explorer is cloned.");
    println!("    Default: $LLVM_AIRFRYER_HOME/compiler-explorer");

    println!("\n  {}  {}", style("LLVM_AIRFRYER_LLVM_SOURCE_PATH").green(), style("(optional)").dim());
    println!("    Default path to the llvm-project source directory.");
    println!("    Overrides the value saved in config.toml.");

    println!("\n{}", style("CONFIG FILE").yellow().bold());
    println!("  {}", config_path().display());
    println!("  Stores persistent settings in TOML format:");
    println!("    {} — remembered llvm-project path", style("llvm_source_path").green());

    println!("\n{}", style("QUICK START").yellow().bold());
    println!("  1. {} Compiler Explorer", style("Download").bold());
    println!("  2. {} LLVM Upstream (main branch)", style("Build").bold());
    println!("  3. {} Compiler Explorer — your custom clang appears automatically", style("Run").bold());
    println!("  4. Use {} to compare assembly across targets", style("CE Flag Presets").bold());

    println!("\n{}", style("KEYBOARD SHORTCUTS").yellow().bold());
    println!("  {}  — navigate menus", style("↑/↓").bold());
    println!("  {}  — type to fuzzy-search menu items", style("a-z").bold());
    println!("  {} — go back / quit", style("Esc").bold());
    println!();
}

fn pause_and_continue(home: &PathBuf) -> bool {
    println!();
    let choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Return to the main menu?")
        .items(&["Yes", "No"])
        .default(0)
        .interact()
        .unwrap_or(1);
    if choice == 1 {
        println!("Goodbye!");
        return false;
    }
    let _ = Term::stdout().clear_screen();
    print_header(home);
    true
}

fn print_header(home: &PathBuf) {
    let version = env!("CARGO_PKG_VERSION");
    println!("🔥 LLVM Airfryer {} — LLVM compiler framework development toolkit",
        style(format!("(v{version})")).dim());
    println!("   {}:   {}", style("Home").dim(), style(home.display()).dim());
    println!("   {}:  {}", style("Builds").dim(), style(builds_dir().display()).dim());
    println!("   {}:      {}\n", style("CE").dim(), style(ce_dir().display()).dim());
}

fn main() {
    let home = airfryer_home();
    print_header(&home);

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

        match idx {
            0 => { download_compiler_explorer(); }
            1 => { run_compiler_explorer(); }
            2 => { stop_compiler_explorer(); }
            3 => { build_llvm_upstream(); }
            4 => { build_llvm_branch(); }
            5 => { build_zig_custom_llvm(); }
            6 => { show_flag_presets(); }
            7 => { resources::show_resources(); }
            8 => { show_help(); }
            9 => {
                println!("Goodbye!");
                break;
            }
            _ => unreachable!(),
        };

        if !pause_and_continue(&home) {
            break;
        }
    }
}
