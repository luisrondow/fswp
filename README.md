# File Tinder 🗂️💘

A fast, keyboard-centric terminal application for decluttering directories using a "Tinder-like" swipe interface. Review files one by one and make rapid decisions to **Keep** or **Trash** them, with rich previews directly in your terminal.

![Rust](https://img.shields.io/badge/Rust-1.70+-orange?logo=rust)
![License](https://img.shields.io/badge/license-MIT-blue)

## ✨ Features

- **Swipe-style interface** — Focus on one file at a time, maximizing screen space for previews
- **Rich previews** — Syntax-highlighted code, images rendered in terminal, PDF first-page previews
- **Safe deletion** — Files go to system Trash, not permanent deletion
- **Undo support** — Made a mistake? Instantly restore the last trashed file
- **Dry-run mode** — Preview what would happen without actually moving files
- **Flexible filtering** — Filter by file type, size range, include hidden files
- **Customizable sorting** — Sort by date, name, size, or type
- **Responsive UI** — Async preview loading keeps the interface snappy

## 📦 Installation

### From Source

```bash
git clone https://github.com/luisrondow/dating-files.git
cd dating-files
cargo build --release
```

The binary will be available at `target/release/file-tinder`.

### Optional: PDF Support

For PDF preview support, install the Pdfium library:

**macOS (Homebrew):**
```bash
brew install pdfium
```

**Linux:**
Download from [pdfium-binaries](https://github.com/AprliRawormd/pdfium-binaries/releases) and add to your library path.

## 🚀 Usage

```
File Tinder - A terminal-based file decluttering tool

Usage: file-tinder [OPTIONS] [DIRECTORY]

Arguments:
  [DIRECTORY]
          Directory to scan for files
          
          If not specified, defaults to the current directory.
          
          [default: .]

Options:
  -t, --type <FILE_TYPES>
          Filter by file type(s)
          
          Can be specified multiple times to include multiple types.
          Example: --type text --type image

          Possible values:
          - text:   Text files (txt, md, rs, py, js, etc.)
          - image:  Image files (png, jpg, gif, etc.)
          - pdf:    PDF files
          - binary: Binary/other files

  -n, --dry-run
          Dry run mode - preview actions without actually moving files to trash
          
          In dry run mode, no files will be moved or deleted.
          Useful for testing or seeing what would happen.

  -s, --sort <SORT_BY>
          Sort files by specified criteria

          Possible values:
          - date: Sort by modification date (oldest first)
          - name: Sort by file name (alphabetical)
          - size: Sort by file size (smallest first)
          - type: Sort by file type
          
          [default: date]

  -r, --reverse
          Reverse sort order

      --hidden
          Show hidden files (files starting with .)

      --min-size <MIN_SIZE>
          Minimum file size filter (e.g., "1KB", "5MB", "1GB")

      --max-size <MAX_SIZE>
          Maximum file size filter (e.g., "100MB", "1GB")

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

### Examples

```bash
# Review all files in the current directory
file-tinder

# Review files in a specific directory
file-tinder ~/Downloads

# Review only text files
file-tinder --type text ~/Documents

# Review images and PDFs
file-tinder -t image -t pdf ~/Pictures

# Dry run - see what would happen without moving files
file-tinder --dry-run ~/Downloads

# Review large images only (over 5MB)
file-tinder --type image --min-size 5MB ~/Photos

# Review old files first (sorted by date)
file-tinder --sort date ~/Archive

# Review newest files first
file-tinder --sort date --reverse .

# Include hidden files, sorted by name
file-tinder --hidden --sort name ~/config

# Find files between 1MB and 100MB
file-tinder --min-size 1MB --max-size 100MB ~/Downloads
```

## ⌨️ Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `→` / `l` | **Keep** — Skip to next file |
| `←` / `h` | **Trash** — Move file to system trash |
| `↑` / `k` | Scroll preview up |
| `↓` / `j` | Scroll preview down |
| `u` / `Backspace` | **Undo** — Restore last trashed file |
| `?` | Toggle help overlay |
| `q` / `Esc` | Quit application |

## 📄 Supported File Types

| Type | Extensions | Preview |
|------|------------|---------|
| **Text/Code** | `.txt`, `.md`, `.rs`, `.py`, `.js`, `.ts`, `.json`, `.toml`, `.yaml`, `.html`, `.css`, etc. | Syntax-highlighted content |
| **Images** | `.png`, `.jpg`, `.jpeg`, `.gif`, `.webp`, `.bmp` | Terminal-rendered image (Sixel/Kitty/ASCII fallback) |
| **PDF** | `.pdf` | First page rendered as image |
| **Binary** | Other files | File metadata display |

## 🏗️ Tech Stack

- **[Rust](https://www.rust-lang.org/)** — Performance and safety
- **[ratatui](https://ratatui.rs/)** — Terminal UI framework
- **[crossterm](https://github.com/crossterm-rs/crossterm)** — Cross-platform terminal handling
- **[tokio](https://tokio.rs/)** — Async runtime for responsive UI
- **[trash](https://crates.io/crates/trash)** — Safe system trash integration
- **[syntect](https://github.com/trishume/syntect)** — Syntax highlighting
- **[ratatui-image](https://crates.io/crates/ratatui-image)** — Terminal image rendering
- **[pdfium-render](https://crates.io/crates/pdfium-render)** — PDF rendering
- **[clap](https://clap.rs/)** — CLI argument parsing

## 📝 License

MIT License — see [LICENSE](LICENSE) for details.

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
