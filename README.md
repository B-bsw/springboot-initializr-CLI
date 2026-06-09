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

springx remove web                    # remove web dependency
springx remove web,data-jpa           # remove multiple dependencies
```

### Self-update

```bash
springx update         # update springx CLI to the latest version
```

## Flags reference

| Flag | Short | Description |
|------|-------|-------------|
| `--project` | `-t` | Project type (`maven-project`, `gradle-project`) |
| `--language` | `-l` | Language (`java`, `kotlin`, `groovy`) |
| `--boot` | `-b` | Spring Boot version |
| `--name` | `-n` | Project name |
| `--group` | `-g` | Group ID |
| `--artifact` | `-a` | Artifact ID |
| `--package-name` | | Package name |
| `--packaging` | `-p` | `jar` or `war` |
| `--java` | `-j` | Java version |
| `--config-format` | `-f` | `properties` or `yaml` |
| `--deps` | `-d` | Comma-separated dependency IDs |
| `--output` | `-o` | Output directory (default: `.`) |
| `--ide` | | Open in IDE after generation |
| `--flat` | | Extract without wrapper folder |

## Build from source

```bash
cd springboot-initializr-CLI
cargo build --release
# Binary at: springboot-initializr-CLI/target/release/springx
```
