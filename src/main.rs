use clap::{Parser, Subcommand};
use colored::*;
use dialoguer::{Confirm, Input, Select};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, Stdio};
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Server '{0}' not found")]
    ServerNotFound(String),
    #[error("Server '{0}' already exists")]
    ServerExists(String),
    #[error("Docker not installed")]
    DockerNotInstalled,
    #[error("Failed to parse config: {0}")]
    ConfigParse(#[from] serde_json::Error),
    #[error("Failed to parse yaml: {0}")]
    YamlError(#[from] serde_yaml::Error),
    #[error("Invalid server name: {0}")]
    InvalidServerName(String),
    #[error("Docker command failed: {0}")]
    DockerCommandFailed(String),
    #[error("Dialog error: {0}")]
    DialogError(#[from] dialoguer::Error),
}

type Result<T> = std::result::Result<T, ServerError>;

#[derive(Parser)]
#[command(
    name = "mc-server",
    about = "A production-ready CLI tool for managing multiple Minecraft servers with Docker",
    version = "1.0.0",
    author = "Your Name"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new Minecraft server
    Create,
    /// List all servers
    List,
    /// Start specific server(s)
    Start {
        /// Server name (optional, starts all if not specified)
        name: Option<String>,
    },
    /// Stop specific server(s)
    Stop {
        /// Server name (optional, stops all if not specified)
        name: Option<String>,
    },
    /// Show server logs
    Logs {
        /// Server name
        name: String,
        /// Follow logs in real-time
        #[arg(short, long)]
        follow: bool,
    },
    /// Remove a server
    Remove {
        /// Server name
        name: String,
        /// Force removal without confirmation
        #[arg(short, long)]
        force: bool,
    },
    /// Attach to server console
    Console {
        /// Server name
        name: String,
    },
    /// List available versions and types
    Versions,
    /// Backup server data
    Backup {
        /// Server name
        name: String,
    },
    /// Restore server from backup
    Restore {
        /// Server name
        name: String,
        /// Backup file path
        path: PathBuf,
    },
}

#[derive(Serialize, Deserialize, Clone)]
struct ServerConfig {
    servers: HashMap<String, ServerInfo>,
}

#[derive(Serialize, Deserialize, Clone)]
struct ServerInfo {
    version: String,
    port: String,
    memory: String,
    data_path: String,
    server_type: String,
    mod_loader: Option<String>,
    mod_loader_version: Option<String>,
    java_args: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    last_started: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Serialize, Deserialize)]
struct ComposeConfig {
    version: String,
    services: HashMap<String, MinecraftService>,
}

#[derive(Serialize, Deserialize)]
struct MinecraftService {
    image: String,
    container_name: String,
    ports: Vec<String>,
    environment: Vec<String>,
    volumes: Vec<String>,
    restart: String,
    stdin_open: bool,
    tty: bool,
}

const CONFIG_DIR: &str = ".mc-servers";
const CONFIG_FILE: &str = "servers.json";
const BACKUP_DIR: &str = "backups";

fn main() -> Result<()> {
    print_banner();
    let cli = Cli::parse();

    ensure_config_dir()?;

    if !check_docker_installed() {
        install_docker()?;
    }

    match cli.command.unwrap_or(Commands::List) {
        Commands::Create => create_server()?,
        Commands::List => list_servers()?,
        Commands::Start { name } => start_servers(name)?,
        Commands::Stop { name } => stop_servers(name)?,
        Commands::Logs { name, follow } => show_logs(&name, follow)?,
        Commands::Remove { name, force } => remove_server(&name, force)?,
        Commands::Console { name } => attach_console(&name)?,
        Commands::Versions => list_versions(),
        Commands::Backup { name } => backup_server(&name)?,
        Commands::Restore { name, path } => restore_server(&name, &path)?,
    }

    Ok(())
}

fn print_banner() {
    println!("{}", r#"
╔╦╗╔═╗  ╔═╗╔═╗╦═╗╦  ╦╔═╗╦═╗
║║║║    ╚═╗║╣ ╠╦╝╚╗╔╝║╣ ╠╦╝
╩ ╩╚═╝  ╚═╝╚═╝╩╚═ ╚╝ ╚═╝╩╚═
    "#.bright_green());
    println!("{}", "Production Minecraft Server Manager".bright_blue().bold());
    println!("{}", "=================================".bright_blue());
}

fn ensure_config_dir() -> Result<()> {
    let config_dir = Path::new(CONFIG_DIR);
    if !config_dir.exists() {
        fs::create_dir_all(config_dir)?;
        fs::create_dir_all(config_dir.join(BACKUP_DIR))?;
        save_server_config(&ServerConfig {
            servers: HashMap::new(),
        })?;
    }
    Ok(())
}

fn load_server_config() -> Result<ServerConfig> {
    let config_path = Path::new(CONFIG_DIR).join(CONFIG_FILE);
    if config_path.exists() {
        let content = fs::read_to_string(config_path)?;
        Ok(serde_json::from_str(&content)?)
    } else {
        Ok(ServerConfig {
            servers: HashMap::new(),
        })
    }
}

fn save_server_config(config: &ServerConfig) -> Result<()> {
    let config_path = Path::new(CONFIG_DIR).join(CONFIG_FILE);
    let content = serde_json::to_string_pretty(config)?;
    fs::write(config_path, content)?;
    Ok(())
}

fn check_docker_installed() -> bool {
    ProcessCommand::new("docker")
        .arg("--version")
        .output()
        .is_ok()
}

fn install_docker() -> Result<()> {
    println!("{}", "\nDocker not found! Installing Docker...".yellow());
    let pb = create_spinner("Installing Docker");

    #[cfg(target_os = "linux")]
    {
        pb.set_message("Installing Docker on Linux...");
        let output = ProcessCommand::new("sh")
            .arg("-c")
            .arg("curl -fsSL https://get.docker.com | sh")
            .output()?;

        if !output.status.success() {
            return Err(ServerError::DockerCommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }
    }

    #[cfg(target_os = "macos")]
    {
        pb.set_message("Installing Docker on macOS...");
        let output = ProcessCommand::new("brew")
            .arg("install")
            .arg("docker")
            .output()?;

        if !output.status.success() {
            return Err(ServerError::DockerCommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }
    }

    #[cfg(target_os = "windows")]
    {
        pb.set_message("Installing Docker on Windows...");
        println!("{}", "\nPlease download and install Docker Desktop from:".yellow());
        println!("https://www.docker.com/products/docker-desktop");
        if !Confirm::new().with_prompt("Have you installed Docker Desktop?").interact()? {
            return Err(ServerError::DockerNotInstalled);
        }
    }

    pb.finish_with_message("Docker installed successfully!");
    Ok(())
}

fn create_spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

fn list_versions() {
    println!("\n{}", "Available Minecraft Server Types:".bright_cyan());
    println!("{}", "============================".bright_cyan());
    println!("- {}: Vanilla Minecraft server", "VANILLA".bright_green());
    println!("- {}: High performance fork of Spigot", "PAPER".bright_green());
    println!("- {}: Modded Minecraft server", "FORGE".bright_green());
    println!("- {}: Lightweight mod loader", "FABRIC".bright_green());
    println!("- {}: Fork of CraftBukkit", "SPIGOT".bright_green());
    println!("- {}: Performance-focused server", "PURPUR".bright_green());

    println!("\n{}", "Version Format Examples:".bright_yellow());
    println!("- LATEST (always uses the latest release)");
    println!("- 1.20.2 (specific version)");
    println!("- SNAPSHOT (latest snapshot version)");
    
    println!("\n{}", "Mod Loader Examples:".bright_yellow());
    println!("- Forge: RECOMMENDED or specific version (e.g., 47.1.0)");
    println!("- Fabric: LATEST or specific version (e.g., 0.14.21)");
}

fn create_server() -> Result<()> {
    println!("\n{}", "Let's configure a new Minecraft server!".bright_cyan());

    // Server Name
    let server_name: String = Input::new()
        .with_prompt("Enter server name (alphanumeric only)")
        .validate_with(|input: &String| {
            if input.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
                Ok(())
            } else {
                Err("Server name must be alphanumeric")
            }
        })
        .interact_text()?;

    let mut config = load_server_config()?;
    if config.servers.contains_key(&server_name) {
        return Err(ServerError::ServerExists(server_name));
    }

    // Server Type Selection
    let server_types = vec!["VANILLA", "PAPER", "FORGE", "FABRIC", "SPIGOT", "PURPUR"];
    let server_type_idx = Select::new()
        .with_prompt("Select server type")
        .items(&server_types)
        .default(0)
        .interact()?;
    
    let server_type = server_types[server_type_idx];

    // Version Input
    let version: String = Input::new()
        .with_prompt("Enter Minecraft version (e.g., LATEST, 1.20.2, SNAPSHOT)")
        .default("LATEST".into())
        .interact_text()?;

    // Mod Loader Configuration
    let (mod_loader, mod_loader_version) = match server_type {
        "FORGE" => {
            let version = Input::new()
                .with_prompt("Enter Forge version (e.g., 47.1.0, RECOMMENDED)")
                .default("RECOMMENDED".into())
                .interact_text()?;
            (Some("FORGE".to_string()), Some(version))
        },
        "FABRIC" => {
            let version = Input::new()
                .with_prompt("Enter Fabric Loader version (e.g., 0.14.21, LATEST)")
                .default("LATEST".into())
                .interact_text()?;
            (Some("FABRIC".to_string()), Some(version))
        },
        _ => (None, None)
    };

    // Memory Configuration
    let memory: String = Input::new()
        .with_prompt("Enter server memory (e.g., 2G, 4G)")
        .default("2G".into())
        .interact_text()?;

    // Server Port
    let port: String = Input::new()
        .with_prompt("Enter server port")
        .default("25565".into())
        .interact_text()?;

    // Java Arguments (Optional)
    let java_args: Option<String> = if Confirm::new()
        .with_prompt("Would you like to customize Java arguments?")
        .interact()?
    {
        Some(
            Input::new()
                .with_prompt("Enter custom Java arguments")
                .default("-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200".into())
                .interact_text()?,
        )
    } else {
        None
    };

    // EULA Agreement
    if !Confirm::new()
        .with_prompt("Do you agree to the Minecraft EULA? (https://account.mojang.com/documents/minecraft_eula)")
        .interact()?
    {
        println!("{}", "EULA must be accepted to continue.".red());
        return Ok(());
    }

    // Create server directory and compose file
    let data_path = Path::new(CONFIG_DIR).join(&server_name);
    fs::create_dir_all(&data_path)?;

    let mut environment = vec![
        "EULA=TRUE".to_string(),
        format!("MEMORY={}", memory),
        format!("VERSION={}", version),
        format!("TYPE={}", server_type),
    ];

    // Add mod loader configuration if applicable
    if let Some(loader) = &mod_loader {
        environment.push(format!("TYPE={}", loader));
        if let Some(loader_version) = &mod_loader_version {
            match loader.as_str() {
                "FORGE" => environment.push(format!("FORGE_VERSION={}", loader_version)),
                "FABRIC" => environment.push(format!("FABRIC_VERSION={}", loader_version)),
                _ => {}
            }
        }
    }

    // Add Java arguments if specified
    if let Some(args) = &java_args {
        environment.push(format!("JVM_OPTS={}", args));
    }

    let compose_config = ComposeConfig {
        version: "3.8".to_string(),
        services: {
            let mut services = HashMap::new();
            services.insert(
                server_name.clone(),
                MinecraftService {
                    image: "itzg/minecraft-server".to_string(),
                    container_name: format!("mc-{}", server_name),
                    ports: vec![format!("{}:25565", port)],
                    environment,
                    volumes: vec![
                        format!("{}:/data", data_path.to_string_lossy()),
                    ],
                    restart: "unless-stopped".to_string(),
                    stdin_open: true,
                    tty: true,
                },
            );
            services
        },
    };

    // Save configurations
    let compose_path = data_path.join("docker-compose.yml");
    let yaml = serde_yaml::to_string(&compose_config)?;
    fs::write(compose_path, yaml)?;

    // Update server config
    config.servers.insert(
        server_name.clone(),
        ServerInfo {
            version,
            port,
            memory,
            data_path: data_path.to_string_lossy().to_string(),
            server_type: server_type.to_string(),
            mod_loader,
            mod_loader_version,
            java_args,
            created_at: chrono::Utc::now(),
            last_started: None,
        },
    );
    save_server_config(&config)?;

    println!("{}", "\nServer configuration saved successfully!".green());
    if Confirm::new()
        .with_prompt("Would you like to start the server now?")
        .interact()?
    {
        start_servers(Some(server_name))?;
    }

    Ok(())
}

fn list_servers() -> Result<()> {
    let config = load_server_config()?;
    if config.servers.is_empty() {
        println!("{}", "\nNo servers configured yet. Use 'create' to add a server.".yellow());
        return Ok(());
    }

    println!("\n{}", "Configured Minecraft Servers:".bright_cyan());
    println!("{}", "=========================".bright_cyan());

    for (name, info) in config.servers {
        let status = get_server_status(&name)?;
        let mod_info = info.mod_loader.map_or("".to_string(), |m| format!(" ({})", m));
        
        println!(
            "{}: {} {}\n  Version: {}{}\n  Port: {}, Memory: {}\n  Created: {}\n  Last Started: {}\n",
            name.bright_green(),
            status,
            info.server_type.bright_blue(),
            info.version.bright_blue(),
            mod_info.bright_blue(),
            info.port,
            info.memory,
            info.created_at.format("%Y-%m-%d %H:%M:%S"),
            info.last_started.map_or("Never".to_string(), |d| d.format("%Y-%m-%d %H:%M:%S").to_string())
        );
    }

    Ok(())
}

fn get_server_status(name: &str) -> Result<ColoredString> {
    let output = ProcessCommand::new("docker")
        .args(["ps", "-q", "-f", &format!("name=mc-{}", name)])
        .output()?;

    Ok(if !output.stdout.is_empty() {
        "RUNNING".bright_green()
    } else {
        "STOPPED".red()
    })
}

fn start_servers(name: Option<String>) -> Result<()> {
    let config = load_server_config()?;
    let pb = create_spinner("Starting server(s)");

    match name {
        Some(server_name) => {
            if let Some(info) = config.servers.get(&server_name) {
                start_single_server(&server_name, &info.data_path, &pb)?;
                update_last_started(&server_name)?;
            } else {
                return Err(ServerError::ServerNotFound(server_name));
            }
        }
        None => {
            if config.servers.is_empty() {
                println!("{}", "No servers configured!".yellow());
                return Ok(());
            }
            for (name, info) in config.servers {
                start_single_server(&name, &info.data_path, &pb)?;
                update_last_started(&name)?;
            }
        }
    }
    pb.finish_with_message("Server start operation completed!");
    Ok(())
}

fn update_last_started(name: &str) -> Result<()> {
    let mut config = load_server_config()?;
    if let Some(info) = config.servers.get_mut(name) {
        info.last_started = Some(chrono::Utc::now());
        save_server_config(&config)?;
    }
    Ok(())
}

fn start_single_server(name: &str, path: &str, pb: &ProgressBar) -> Result<()> {
    pb.set_message(format!("Starting server {}...", name));
    let output = ProcessCommand::new("docker-compose")
        .current_dir(path)
        .arg("up")
        .arg("-d")
        .output()?;

    if !output.status.success() {
        return Err(ServerError::DockerCommandFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    println!("{}", format!("Server '{}' started successfully!", name).green());
    Ok(())
}

fn stop_servers(name: Option<String>) -> Result<()> {
    let config = load_server_config()?;
    let pb = create_spinner("Stopping server(s)");

    match name {
        Some(server_name) => {
            if let Some(info) = config.servers.get(&server_name) {
                stop_single_server(&server_name, &info.data_path, &pb)?;
            } else {
                return Err(ServerError::ServerNotFound(server_name));
            }
        }
        None => {
            if config.servers.is_empty() {
                println!("{}", "No servers configured!".yellow());
                return Ok(());
            }
            for (name, info) in config.servers {
                stop_single_server(&name, &info.data_path, &pb)?;
            }
        }
    }
    pb.finish_with_message("Server stop operation completed!");
    Ok(())
}

fn stop_single_server(name: &str, path: &str, pb: &ProgressBar) -> Result<()> {
    pb.set_message(format!("Stopping server {}...", name));
    let output = ProcessCommand::new("docker-compose")
        .current_dir(path)
        .arg("down")
        .output()?;

    if !output.status.success() {
        return Err(ServerError::DockerCommandFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    println!("{}", format!("Server '{}' stopped successfully!", name).green());
    Ok(())
}

fn show_logs(name: &str, follow: bool) -> Result<()> {
    let config = load_server_config()?;
    if let Some(info) = config.servers.get(name) {
        println!("{}", format!("\nShowing logs for server '{}':", name).bright_cyan());
        
        let mut cmd = ProcessCommand::new("docker-compose");
        cmd.current_dir(&info.data_path)
            .arg("logs");
        
        if follow {
            println!("{}", "Press Ctrl+C to exit".bright_yellow());
            cmd.arg("-f");
        }

        let status = cmd.status()?;
        if !status.success() {
            return Err(ServerError::DockerCommandFailed("Failed to show logs".to_string()));
        }
    } else {
        return Err(ServerError::ServerNotFound(name.to_string()));
    }
    Ok(())
}

fn attach_console(name: &str) -> Result<()> {
    let config = load_server_config()?;
    if let Some(_) = config.servers.get(name) {
        println!("{}", format!("\nAttaching to server '{}' console:", name).bright_cyan());
        println!("{}", "Type 'exit' or press Ctrl+P, Ctrl+Q to detach".bright_yellow());
        
        let status = ProcessCommand::new("docker")
            .args(["attach", &format!("mc-{}", name)])
            .status()?;

        if !status.success() {
            return Err(ServerError::DockerCommandFailed("Failed to attach to console".to_string()));
        }
    } else {
        return Err(ServerError::ServerNotFound(name.to_string()));
    }
    Ok(())
}

fn backup_server(name: &str) -> Result<()> {
    let config = load_server_config()?;
    if let Some(info) = config.servers.get(name) {
        let pb = create_spinner("Creating backup");
        
        let backup_dir = Path::new(CONFIG_DIR).join(BACKUP_DIR);
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let backup_file = backup_dir.join(format!("{}_{}.tar.gz", name, timestamp));

        // Create tar.gz archive
        let output = ProcessCommand::new("tar")
            .current_dir(&info.data_path)
            .args(["-czf", backup_file.to_str().unwrap(), "."])
            .output()?;

        if !output.status.success() {
            return Err(ServerError::DockerCommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        pb.finish_with_message(format!("Backup created: {}", backup_file.display()));
    } else {
        return Err(ServerError::ServerNotFound(name.to_string()));
    }
    Ok(())
}

fn restore_server(name: &str, backup_path: &Path) -> Result<()> {
    let config = load_server_config()?;
    if let Some(info) = config.servers.get(name) {
        if !backup_path.exists() {
            return Err(ServerError::Io(io::Error::new(
                io::ErrorKind::NotFound,
                "Backup file not found",
            )));
        }

        // Stop server if running
        stop_servers(Some(name.to_string()))?;

        let pb = create_spinner("Restoring backup");

        // Extract backup
        let output = ProcessCommand::new("tar")
            .current_dir(&info.data_path)
            .args(["-xzf", backup_path.to_str().unwrap()])
            .output()?;

        if !output.status.success() {
            return Err(ServerError::DockerCommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        pb.finish_with_message("Backup restored successfully!");

        if Confirm::new()
            .with_prompt("Would you like to start the server now?")
            .interact()?
        {
            start_servers(Some(name.to_string()))?;
        }
    } else {
        return Err(ServerError::ServerNotFound(name.to_string()));
    }
    Ok(())
}

fn remove_server(name: &str, force: bool) -> Result<()> {
    let mut config = load_server_config()?;
    if let Some(info) = config.servers.get(name) {
        if !force && !Confirm::new()
            .with_prompt(format!("Are you sure you want to remove server '{}'? This will delete all data!", name))
            .interact()?
        {
            return Ok(());
        }

        // Stop the server first
        stop_servers(Some(name.to_string()))?;

        // Remove the server directory
        fs::remove_dir_all(&info.data_path)?;

        // Remove from config
        config.servers.remove(name);
        save_server_config(&config)?;

        println!("{}", format!("Server '{}' removed successfully!", name).green());
    } else {
        return Err(ServerError::ServerNotFound(name.to_string()));
    }
    Ok(())
}