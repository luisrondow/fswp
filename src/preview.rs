// Preview module for generating file previews with syntax highlighting
#![allow(dead_code)]

use crate::domain::FileEntry;
use std::fs;
use std::io;
use std::path::Path;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

const MAX_PREVIEW_LINES: usize = 50;

/// Detects the syntax name from a file extension
pub fn detect_syntax_from_extension(extension: &str) -> Option<String> {
    let syntax_set = SyntaxSet::load_defaults_newlines();

    syntax_set
        .find_syntax_by_extension(extension)
        .map(|syntax| syntax.name.clone())
}

/// Reads the first N lines of a text file
pub fn read_file_lines(path: &Path, max_lines: usize) -> io::Result<Vec<String>> {
    let content = fs::read_to_string(path)?;
    let lines: Vec<String> = content
        .lines()
        .take(max_lines)
        .map(|s| s.to_string())
        .collect();

    Ok(lines)
}

/// Generates a syntax-highlighted preview for a text file
pub fn generate_text_preview(file_entry: &FileEntry) -> io::Result<Vec<String>> {
    // Read file content with line limit
    let lines = read_file_lines(&file_entry.path, MAX_PREVIEW_LINES)?;

    // Try to detect syntax from extension
    let extension = file_entry
        .path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let syntax_set = SyntaxSet::load_defaults_newlines();
    let theme_set = ThemeSet::load_defaults();

    let syntax = syntax_set
        .find_syntax_by_extension(extension)
        .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

    let mut highlighter = HighlightLines::new(syntax, &theme_set.themes["base16-ocean.dark"]);

    let mut highlighted_lines = Vec::new();

    for line in lines {
        let line_with_newline = format!("{}\n", line);
        let ranges = highlighter
            .highlight_line(&line_with_newline, &syntax_set)
            .map_err(|e| io::Error::other(format!("Syntax highlighting error: {}", e)))?;

        // Convert highlighted ranges to a displayable string
        let mut line_str = String::new();
        for (_style, text) in ranges {
            // For now, just append the text (color info in style can be used later)
            line_str.push_str(text);
        }

        // Remove trailing newline we added
        if line_str.ends_with('\n') {
            line_str.pop();
        }

        highlighted_lines.push(line_str);
    }

    Ok(highlighted_lines)
}

/// Generates a preview for any file type
pub fn generate_preview(file_entry: &FileEntry) -> io::Result<Vec<String>> {
    use crate::domain::FileType;

    match file_entry.file_type {
        FileType::Text => generate_text_preview(file_entry),
        FileType::Binary => Ok(vec![
            format!("Binary file: {}", file_entry.name),
            format!("Size: {} bytes", file_entry.size),
            String::new(),
            "[Binary content not displayed]".to_string(),
        ]),
        FileType::Image => Ok(vec![
            format!("Image file: {}", file_entry.name),
            format!("Size: {} bytes", file_entry.size),
            String::new(),
            "[Image preview not yet implemented]".to_string(),
        ]),
        FileType::Pdf => Ok(vec![
            format!("PDF file: {}", file_entry.name),
            format!("Size: {} bytes", file_entry.size),
            String::new(),
            "[PDF preview not yet implemented]".to_string(),
        ]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::FileType;
    use chrono::Utc;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    #[test]
    fn test_detect_syntax_from_extension_rust() {
        let syntax = detect_syntax_from_extension("rs");
        assert!(syntax.is_some());
        assert_eq!(syntax.unwrap(), "Rust");
    }

    #[test]
    fn test_detect_syntax_from_extension_python() {
        let syntax = detect_syntax_from_extension("py");
        assert!(syntax.is_some());
        assert_eq!(syntax.unwrap(), "Python");
    }

    #[test]
    fn test_detect_syntax_from_extension_javascript() {
        let syntax = detect_syntax_from_extension("js");
        assert!(syntax.is_some());
        assert_eq!(syntax.unwrap(), "JavaScript");
    }

    #[test]
    fn test_detect_syntax_from_extension_unknown() {
        let syntax = detect_syntax_from_extension("xyz123");
        assert!(syntax.is_none());
    }

    #[test]
    fn test_read_file_lines_basic() {
        let temp_file = NamedTempFile::new().unwrap();
        let content = "line 1\nline 2\nline 3\n";
        fs::write(temp_file.path(), content).unwrap();

        let lines = read_file_lines(temp_file.path(), 10).unwrap();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "line 1");
        assert_eq!(lines[1], "line 2");
        assert_eq!(lines[2], "line 3");
    }

    #[test]
    fn test_read_file_lines_with_limit() {
        let temp_file = NamedTempFile::new().unwrap();
        let content = "line 1\nline 2\nline 3\nline 4\nline 5\n";
        fs::write(temp_file.path(), content).unwrap();

        let lines = read_file_lines(temp_file.path(), 3).unwrap();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "line 1");
        assert_eq!(lines[1], "line 2");
        assert_eq!(lines[2], "line 3");
    }

    #[test]
    fn test_read_file_lines_empty() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), "").unwrap();

        let lines = read_file_lines(temp_file.path(), 10).unwrap();
        assert_eq!(lines.len(), 0);
    }

    #[test]
    fn test_read_file_lines_nonexistent() {
        let result = read_file_lines(Path::new("/nonexistent/file.txt"), 10);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_text_preview() {
        let temp_file = NamedTempFile::new().unwrap();
        let rust_code = "fn main() {\n    println!(\"Hello, world!\");\n}\n";
        fs::write(temp_file.path(), rust_code).unwrap();

        let file_entry = FileEntry {
            path: temp_file.path().with_extension("rs"),
            name: "test.rs".to_string(),
            size: rust_code.len() as u64,
            modified_date: Utc::now(),
            file_type: FileType::Text,
        };

        // Copy to the expected path
        fs::write(&file_entry.path, rust_code).unwrap();

        let preview = generate_text_preview(&file_entry).unwrap();
        assert_eq!(preview.len(), 3);
        assert!(preview[0].contains("fn main()"));
        assert!(preview[1].contains("println!"));

        // Clean up
        fs::remove_file(&file_entry.path).ok();
    }

    #[test]
    fn test_generate_text_preview_respects_line_limit() {
        let temp_file = NamedTempFile::new().unwrap();

        // Generate 100 lines
        let mut content = String::new();
        for i in 1..=100 {
            content.push_str(&format!("line {}\n", i));
        }
        fs::write(temp_file.path(), &content).unwrap();

        let file_entry = FileEntry {
            path: temp_file.path().to_path_buf(),
            name: "test.txt".to_string(),
            size: content.len() as u64,
            modified_date: Utc::now(),
            file_type: FileType::Text,
        };

        let preview = generate_text_preview(&file_entry).unwrap();
        assert_eq!(preview.len(), MAX_PREVIEW_LINES);
        assert_eq!(preview[0], "line 1");
        assert_eq!(preview[49], "line 50");
    }

    #[test]
    fn test_generate_preview_binary() {
        let file_entry = FileEntry {
            path: PathBuf::from("test.bin"),
            name: "test.bin".to_string(),
            size: 1024,
            modified_date: Utc::now(),
            file_type: FileType::Binary,
        };

        let preview = generate_preview(&file_entry).unwrap();
        assert!(preview.len() > 0);
        assert!(preview[0].contains("Binary file"));
        assert!(preview.iter().any(|line| line.contains("not displayed")));
    }

    #[test]
    fn test_generate_preview_image() {
        let file_entry = FileEntry {
            path: PathBuf::from("test.png"),
            name: "test.png".to_string(),
            size: 2048,
            modified_date: Utc::now(),
            file_type: FileType::Image,
        };

        let preview = generate_preview(&file_entry).unwrap();
        assert!(preview.len() > 0);
        assert!(preview[0].contains("Image file"));
        assert!(preview
            .iter()
            .any(|line| line.contains("not yet implemented")));
    }

    #[test]
    fn test_generate_preview_pdf() {
        let file_entry = FileEntry {
            path: PathBuf::from("test.pdf"),
            name: "test.pdf".to_string(),
            size: 4096,
            modified_date: Utc::now(),
            file_type: FileType::Pdf,
        };

        let preview = generate_preview(&file_entry).unwrap();
        assert!(preview.len() > 0);
        assert!(preview[0].contains("PDF file"));
        assert!(preview
            .iter()
            .any(|line| line.contains("not yet implemented")));
    }
}
