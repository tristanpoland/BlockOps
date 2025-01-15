# 🎮 BlockOps

<div align="center">

```
╔╦╗╔═╗  ╔═╗╔═╗╦═╗╦  ╦╔═╗╦═╗
║║║║    ╚═╗║╣ ╠╦╝╚╗╔╝║╣ ╠╦╝
╩ ╩╚═╝  ╚═╝╚═╝╩╚═ ╚╝ ╚═╝╩╚═
```

### Production-Ready Minecraft Server Management CLI

*Simplify your Minecraft server management with Docker-powered automation*

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Docker](https://img.shields.io/badge/Docker-Powered-blue.svg)](https://www.docker.com/)
[![Rust](https://img.shields.io/badge/Built%20with-Rust-orange.svg)](https://www.rust-lang.org/)

[Features](#-features) • 
[Installation](#-installation) • 
[Usage](#-usage) • 
[Configuration](#-configuration) • 
[Troubleshooting](#-troubleshooting)

</div>

---

## ✨ Features

| Feature | Description |
|---------|-------------|
| 🚀 **Quick Setup** | Interactive server creation wizard |
| 🔄 **Multiple Types** | Support for Vanilla, Paper, Forge, Fabric, Spigot, Purpur |
| 📦 **Docker Integration** | Automatic container management |
| 💾 **Backup System** | Built-in backup and restore functionality |
| 🎮 **Console Access** | Direct server console interaction |
| 📊 **Monitoring** | Real-time log viewing and server status |
| 🔧 **Customization** | Flexible server configurations |
| 🎯 **Performance** | Memory and optimization options |
| 🔄 **Auto-Restart** | Automatic recovery on system reboot |


## 🚀 Installation

### Prerequisites

- Operating System: Linux, macOS, or Windows
- Docker (automatically installed if missing)
- Rust toolchain (for building from source)

### Quick Start

```bash
# Clone the repository
git clone https://github.com/tristanpoland/BlockOps
cd mc-server-manager

# Build and install
cargo install --path .
```

## 🎮 Usage

### Essential Commands


| Command | Description |
|---------|-------------|
| `mc-server create` | 🆕 Create a new server |
| `mc-server list` | 📋 List all servers |
| `mc-server start [name]` | ▶️ Start server(s) |
| `mc-server stop [name]` | ⏹️ Stop server(s) |
| `mc-server logs <name> [-f]` | 📊 View server logs |
| `mc-server console <name>` | 🎮 Access server console |
| `mc-server backup <name>` | 💾 Create backup |
| `mc-server restore <name> <path>` | 📥 Restore from backup |
| `mc-server versions` | 📜 List available versions |
| `mc-server remove <name>` | 🗑️ Remove server |

### 🎲 Server Types


| Type | Description | Best For |
|------|-------------|----------|
| 🟢 **VANILLA** | Official Minecraft Server | Purists |
| 🚀 **PAPER** | High Performance Fork | Large Servers |
| 🛠️ **FORGE** | Mod Support | Modded Play |
| 🪶 **FABRIC** | Lightweight Mod Loader | Modern Mods |
| 🔧 **SPIGOT** | Plugin Support | Plugin Users |
| ⚡ **PURPUR** | Performance Focused | Optimization |


## ⚙️ Configuration

### Version Format
- `LATEST` → Most recent release
- `1.20.2` → Specific version
- `SNAPSHOT` → Latest snapshot

### 📁 Directory Structure

```
.mc-servers/
├── 📄 servers.json        # Configuration
├── 📁 backups/           # Backup storage
└── 📁 <server-name>/     # Server data
    ├── 📄 docker-compose.yml
    └── 📁 server files...
```

## 🛠️ Troubleshooting

### Common Solutions

<details>
<summary>🔴 Server Won't Start</summary>

1. Check Docker status: `docker ps`
2. View logs: `mc-server logs <name>`
3. Verify port availability
</details>

<details>
<summary>⚡ Performance Issues</summary>

1. Review memory allocation
2. Check Java arguments
3. Consider Paper/Purpur
</details>

<details>
<summary>🐳 Docker Problems</summary>

1. Verify Docker is running and that you have permission to run docker commands
2. Check container logs
3. Restart Docker service
</details>

## 🤝 Contributing

We welcome contributions! Here's how you can help:

1. 🍴 Fork the repository
2. 🔧 Create your feature branch
3. 💻 Commit your changes
4. 🚀 Push to the branch
5. ✨ Open a Pull Request

## 📜 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

<div align="center">

Made with ❤️ by Tristan J. Poland

*Happy Crafting! 🎮*

</div>