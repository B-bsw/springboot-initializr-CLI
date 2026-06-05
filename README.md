# 🍃 spring-init CLI

A fast, native CLI client for [start.spring.io](https://start.spring.io) — generate Spring Boot projects right from your terminal.

Built in **Rust** for speed. Zero runtime dependencies.

## Install

**Via Curl (Recommended):**
```bash
curl -sSL https://raw.githubusercontent.com/B-bsw/springboot-initalizr-CLI/main/install.sh | bash
```

**Via Cargo:**
```bash
git clone https://github.com/B-bsw/springboot-initalizr-CLI.git
cd springboot-initalizr-CLI
cargo install --path .
```

This installs `spring-init` to `~/.cargo/bin/`.

## Uninstall

If you installed via **curl**, you can remove the binary by running:
```bash
rm -f /usr/local/bin/spring-init ~/.local/bin/spring-init
```

If you installed via **cargo**, use:
```bash
cargo uninstall spring-init
```

## Usage

### Interactive wizard (default)

```bash
spring-init
```

Walks you through every option with fuzzy-searchable menus.

### One-liner generation

```bash
spring-init new \
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
spring-init list           # show everything
spring-init list boot      # boot versions
spring-init list deps      # all dependencies (grouped)
spring-init list java      # java versions
spring-init list languages # languages
spring-init list projects  # project types
spring-init list packaging # jar/war
spring-init list config    # properties/yaml
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
cd springboot-initalizr-CLI
cargo build --release
# Binary at: springboot-initalizr-CLI/target/release/spring-init
```
