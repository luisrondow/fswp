# fswp

A fast, keyboard-centric terminal application for decluttering directories using a swipe-style interface. Review files one by one and make rapid decisions to **Keep** or **Trash** them, with rich previews directly in your terminal.

![Rust](https://img.shields.io/badge/Rust-1.70+-orange?logo=rust)
![License](https://img.shields.io/badge/license-MIT-blue)

## Features

- **Swipe-style interface** — Focus on one file at a time, maximizing screen space for previews
- **Rich previews** — Syntax-highlighted code, images rendered in terminal, PDF text extraction
- **Safe deletion** — Files go to system Trash, not permanent deletion
- **Confirmation dialogs** — Confirm before trashing files (can be skipped with `-y`)
- **Undo support** — Made a mistake? Instantly restore the last trashed file
- **Open in editor** — Open files directly in your preferred editor with `o`
- **Dry-run mode** — Preview what would happen without actually moving files
- **Flexible filtering** — Filter by file type, size range, include hidden files
- **Customizable sorting** — Sort by date, name, size, or type
- **Responsive UI** — Async preview loading keeps the interface snappy
- **Welcome dialog** — First-launch guide for new users

## Installation

### From Source

```bash
git clone https://github.com/luisrondow/dating-files.git
cd dating-files
cargo build --release
```

The binary will be available at `target/release/fswp`.

### Optional: PDF Support

For PDF preview support, install the Pdfium library:

**Unix-based:**
Download from [pdfium-binaries](https://github.com/AprliRawormd/pdfium-binaries/releases) and add to your library (`/lib`) path.

## Usage

```
fswp [OPTIONS] [DIRECTORY]

Arguments:
  [DIRECTORY]  Directory to scan for files [default: .]

Options:
  -t, --type <TYPE>       Filter by file type (text, image, pdf, binary)
  -n, --dry-run           Preview actions without moving files to trash
  -s, --sort <SORT>       Sort by criteria (date, name, size, type) [default: date]
  -r, --reverse           Reverse sort order
      --hidden            Show hidden files (files starting with .)
      --min-size <SIZE>   Minimum file size (e.g., "1KB", "5MB", "1GB")
      --max-size <SIZE>   Maximum file size (e.g., "100MB", "1GB")
  -y, --yes               Skip confirmation prompts for trash actions
      --welcome           Show welcome dialog on startup
  -h, --help              Print help
  -V, --version           Print version
```

### Examples

```bash
# Review all files in the current directory
fswp

# Review files in a specific directory
fswp ~/Downloads

# Review only text files
fswp --type text ~/Documents

# Review images and PDFs
fswp -t image -t pdf ~/Pictures

# Dry run - see what would happen without moving files
fswp --dry-run ~/Downloads

# Review large images only (over 5MB)
fswp --type image --min-size 5MB ~/Photos

# Review newest files first
fswp --sort date --reverse .

# Skip confirmation prompts for faster workflow
fswp -y ~/Downloads

# Include hidden files, sorted by name
fswp --hidden --sort name ~/config

# Find files between 1MB and 100MB
fswp --min-size 1MB --max-size 100MB ~/Downloads
```

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `→` / `k` | **Keep** — Leave file in place, move to next |
| `←` / `t` | **Trash** — Move file to system trash |
| `↑` / `i` | **Previous** — Go to previous file |
| `↓` / `j` | **Next** — Go to next file |
| `o` | **Open** — Open file in editor (`$EDITOR` / `$VISUAL` / system default) |
| `u` / `Ctrl+Z` | **Undo** — Restore last trashed file |
| `?` | Toggle help overlay |
| `q` / `Esc` / `Ctrl+C` | Quit application |

### Confirmation Dialog

When trashing a file (unless `-y` flag is used):

| Key | Action |
|-----|--------|
| `y` / `Enter` | Confirm trash |
| `n` / `Esc` | Cancel |

## Supported File Types

| Type | Extensions | Preview |
|------|------------|---------|
| **Text/Code** | `.txt`, `.md`, `.rs`, `.py`, `.js`, `.ts`, `.jsx`, `.tsx`, `.json`, `.yaml`, `.toml`, `.html`, `.css`, `.go`, `.java`, `.c`, `.cpp`, `.sh`, etc. | Syntax-highlighted content |
| **Images** | `.png`, `.jpg`, `.jpeg`, `.gif`, `.webp`, `.bmp` | Half-block character rendering with true color |
| **PDF** | `.pdf` | Text extraction from first page |
| **Binary** | Other files | File metadata display |

## Configuration

User configuration is stored at `~/.config/fswp/config.json`. This tracks whether the welcome dialog has been shown.

## Tech Stack

- **[Rust](https://www.rust-lang.org/)** — Performance and safety
- **[ratatui](https://ratatui.rs/)** — Terminal UI framework
- **[crossterm](https://github.com/crossterm-rs/crossterm)** — Cross-platform terminal handling
- **[tokio](https://tokio.rs/)** — Async runtime for responsive UI
- **[clap](https://clap.rs/)** — CLI argument parsing
- **[trash](https://crates.io/crates/trash)** — Safe system trash integration
- **[syntect](https://github.com/trishume/syntect)** — Syntax highlighting
- **[ratatui-image](https://crates.io/crates/ratatui-image)** — Terminal image rendering
- **[pdfium-render](https://crates.io/crates/pdfium-render)** — PDF rendering
- **[edit](https://crates.io/crates/edit)** — Editor integration
- **[serde](https://serde.rs/)** — Configuration serialization

## License

MIT License — see [LICENSE](LICENSE) for details.

## Contributing

Contributions are welcome. Please feel free to submit a Pull Request.
