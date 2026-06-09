# 🍃 springx CLI

A fast, native CLI client for [start.spring.io](https://start.spring.io) — generate Spring Boot projects right from your terminal.

Built in **Rust** for speed. Zero runtime dependencies.

## Install

**Via Curl (Recommended):**
```bash
curl -sSL https://raw.githubusercontent.com/B-bsw/springboot-initializr-CLI/main/install.sh | bash
```

**Via Cargo:**
```bash
git clone https://github.com/B-bsw/springboot-initializr-CLI.git
cd springboot-initalizr-CLI
cargo install --path .
```

This installs `springx` to `~/.cargo/bin/`.

## Uninstall

If you installed via **curl**, you can remove the binary by running:
```bash
rm -f /usr/local/bin/springx ~/.local/bin/springx
```

If you installed via **cargo**, use:
```bash
cargo uninstall springx
```

## Usage

### Interactive wizard (default)

```bash
springx
```

Walks you through every option with fuzzy-searchable menus.

### One-liner generation

```bash
springx init \
  --name my-api \
  --group com.mycompany \
  --artifact my-api \
  --boot 4.0.6 \
  --language java \
  --java 21 \
  --packaging jar \
  --deps web,data-jpa,validation,security \
  --output ~/projects \
  --ide idea
```

All flags are optional — anything you omit uses the server default.

### List available options

```bash
springx list           # List all available dependencies in a clean, readable format
```

### Dependency management (for existing projects)

```bash
springx add                           # open interactive menu to add dependencies
springx add web                       # add spring-boot-starter-web
springx add web,data-jpa,lombok       # add multiple dependencies (comma-separated)
springx add web data-jpa              # add multiple dependencies (space-separated)

springx update                        # update all existing dependencies to the latest initializr snippets
springx update web                    # update only the web dependency

springx remove web                    # remove web dependency
springx remove web,data-jpa           # remove multiple dependencies
```

### Project Inspection

```bash
springx doctor                        # Validate local development environment (Java, Git, Docker, IDEs, etc.)
springx deps                          # Display dependencies currently installed in the project
springx search security               # Search for dependencies by name or ID
springx info security                 # Get detailed information about a dependency
```

### Self-upgrade

```bash
springx upgrade                       # upgrade the springx CLI itself to the latest version
```

## Flags reference

| Flag | Short | Description |
|------|-------|-------------|
| `--type` | `-t` | Project type (maven-project, gradle-project) |
| `--language` | | Programming language (java, kotlin, groovy) |
| `--boot` | | Spring Boot version |
| `--name` | `-n` | Project name |
| `--group` | | Group ID (e.g. `com.example`) |
| `--artifact` | | Artifact ID |
| `--package-name` | | Package name (e.g. `com.example.demo`) |
| `--packaging` | | Packaging (jar, war) |
| `--java` | | Java version (e.g. 21, 17) |
| `--format` | `-f` | Configuration file format (properties, yaml) |
| `--deps` | `-d` | Comma-separated dependencies (`web,data-jpa`) |
| `--output` | `-o` | Output directory |
| `--flat` | | Extract directly into output dir (no root folder) |
| `--ide` | | Open project in IDE after generation (`idea`, `code`) |
| `--git` | | Initialize a git repository after generation |
| `--docker` | | Generate multi-stage Dockerfile and docker-compose.yml |
| `--template` | | Apply a project template (e.g. `clean-architecture`) |


## Build from source

```bash
cd springboot-initializr-CLI
cargo build --release
# Binary at: springboot-initializr-CLI/target/release/springx
```
