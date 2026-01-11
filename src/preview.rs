// Preview module for generating file previews with syntax highlighting and images
#![allow(dead_code)]

use crate::domain::FileEntry;
use image::{DynamicImage, GenericImageView};
use std::fs;
use std::io;
use std::path::Path;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

const MAX_PREVIEW_LINES: usize = 50;
const MAX_IMAGE_WIDTH: u32 = 80;
const MAX_IMAGE_HEIGHT: u32 = 40;

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

/// Loads an image from a file path
pub fn load_image(path: &Path) -> io::Result<DynamicImage> {
    image::open(path).map_err(|e| io::Error::other(format!("Image loading error: {}", e)))
}

/// Calculates new dimensions to fit image within max width and height while preserving aspect ratio
pub fn calculate_resize_dimensions(
    original_width: u32,
    original_height: u32,
    max_width: u32,
    max_height: u32,
) -> (u32, u32) {
    if original_width == 0 || original_height == 0 {
        return (0, 0);
    }

    let width_ratio = max_width as f64 / original_width as f64;
    let height_ratio = max_height as f64 / original_height as f64;

    let ratio = width_ratio.min(height_ratio);

    if ratio >= 1.0 {
        // Image is smaller than max dimensions, don't upscale
        (original_width, original_height)
    } else {
        // Scale down proportionally
        let new_width = (original_width as f64 * ratio) as u32;
        let new_height = (original_height as f64 * ratio) as u32;
        (new_width, new_height)
    }
}

/// Converts an image to ASCII art for terminal display
pub fn image_to_ascii(img: &DynamicImage, width: u32, height: u32) -> Vec<String> {
    let img = img.resize_exact(width, height, image::imageops::FilterType::Lanczos3);
    let img = img.to_luma8();

    let ascii_chars = " .'`^\",:;Il!i><~+_-?][}{1)(|\\/tfjrxnuvczXYUJCLQ0OZmwqpdbkhao*#MW&8%B@$";
    let ascii_len = ascii_chars.len() as f64;

    let mut lines = Vec::new();
    for y in 0..height {
        let mut line = String::new();
        for x in 0..width {
            let pixel = img.get_pixel(x, y);
            let intensity = pixel[0] as f64 / 255.0;
            let char_index = (intensity * (ascii_len - 1.0)) as usize;
            let ch = ascii_chars.chars().nth(char_index).unwrap_or(' ');
            line.push(ch);
        }
        lines.push(line);
    }

    lines
}

/// Generates an image preview using ASCII art
pub fn generate_image_preview(file_entry: &FileEntry) -> io::Result<Vec<String>> {
    let img = load_image(&file_entry.path)?;
    let (original_width, original_height) = img.dimensions();

    let (new_width, new_height) = calculate_resize_dimensions(
        original_width,
        original_height,
        MAX_IMAGE_WIDTH,
        MAX_IMAGE_HEIGHT,
    );

    let mut preview = vec![
        format!("Image: {}", file_entry.name),
        format!("Dimensions: {}x{} pixels", original_width, original_height),
        format!("Size: {} bytes", file_entry.size),
        String::new(),
    ];

    // Generate ASCII art
    let ascii_art = image_to_ascii(&img, new_width, new_height);
    preview.extend(ascii_art);

    Ok(preview)
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
        FileType::Image => generate_image_preview(file_entry),
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

    // Image preview tests
    #[test]
    fn test_calculate_resize_dimensions_scale_down() {
        // Image larger than max dimensions should be scaled down
        let (width, height) = calculate_resize_dimensions(1600, 1200, 80, 40);
        assert!(width <= 80);
        assert!(height <= 40);
        // Check aspect ratio is approximately preserved (4:3)
        // Allow for rounding errors within 1 pixel
        let ratio_diff = (width * 3).abs_diff(height * 4);
        assert!(
            ratio_diff <= 4,
            "Aspect ratio not preserved: {}x{}",
            width,
            height
        );
    }

    #[test]
    fn test_calculate_resize_dimensions_no_upscale() {
        // Image smaller than max dimensions should not be upscaled
        let (width, height) = calculate_resize_dimensions(40, 30, 80, 40);
        assert_eq!(width, 40);
        assert_eq!(height, 30);
    }

    #[test]
    fn test_calculate_resize_dimensions_zero() {
        // Zero dimensions should return zero
        let (width, height) = calculate_resize_dimensions(0, 0, 80, 40);
        assert_eq!(width, 0);
        assert_eq!(height, 0);
    }

    #[test]
    fn test_calculate_resize_dimensions_wide_image() {
        // Very wide image should be constrained by width
        let (width, height) = calculate_resize_dimensions(1600, 400, 80, 40);
        assert_eq!(width, 80);
        assert_eq!(height, 20); // 1600:400 = 80:20
    }

    #[test]
    fn test_calculate_resize_dimensions_tall_image() {
        // Very tall image should be constrained by height
        let (width, height) = calculate_resize_dimensions(400, 1600, 80, 40);
        assert_eq!(width, 10); // 400:1600 = 10:40
        assert_eq!(height, 40);
    }

    #[test]
    fn test_image_to_ascii_dimensions() {
        // Create a simple white 10x10 image
        let img = DynamicImage::new_rgb8(10, 10);
        let ascii = image_to_ascii(&img, 5, 5);

        assert_eq!(ascii.len(), 5); // 5 rows
        for line in &ascii {
            assert_eq!(line.len(), 5); // 5 characters per row
        }
    }

    #[test]
    fn test_load_image_png() {
        use tempfile::TempDir;

        // Create a temporary directory and a simple PNG image
        let temp_dir = TempDir::new().unwrap();
        let image_path = temp_dir.path().join("test.png");

        // Create a simple 10x10 red image
        let img = image::RgbImage::from_fn(10, 10, |_, _| image::Rgb([255, 0, 0]));
        img.save(&image_path).unwrap();

        // Test loading the image
        let loaded = load_image(&image_path).unwrap();
        assert_eq!(loaded.dimensions(), (10, 10));
    }

    #[test]
    fn test_load_image_jpg() {
        use tempfile::TempDir;

        // Create a temporary directory and a simple JPEG image
        let temp_dir = TempDir::new().unwrap();
        let image_path = temp_dir.path().join("test.jpg");

        // Create a simple 10x10 blue image
        let img = image::RgbImage::from_fn(10, 10, |_, _| image::Rgb([0, 0, 255]));
        img.save(&image_path).unwrap();

        // Test loading the image
        let loaded = load_image(&image_path).unwrap();
        assert_eq!(loaded.dimensions(), (10, 10));
    }

    #[test]
    fn test_load_image_nonexistent() {
        let result = load_image(Path::new("/nonexistent/image.png"));
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_image_preview_with_real_image() {
        use tempfile::TempDir;

        // Create a temporary directory and a simple image
        let temp_dir = TempDir::new().unwrap();
        let image_path = temp_dir.path().join("test.png");

        // Create a simple 100x100 gradient image
        let img = image::RgbImage::from_fn(100, 100, |x, y| {
            let intensity = ((x + y) * 255 / 200) as u8;
            image::Rgb([intensity, intensity, intensity])
        });
        img.save(&image_path).unwrap();

        let file_entry = FileEntry {
            path: image_path.clone(),
            name: "test.png".to_string(),
            size: fs::metadata(&image_path).unwrap().len(),
            modified_date: Utc::now(),
            file_type: FileType::Image,
        };

        let preview = generate_image_preview(&file_entry).unwrap();

        // Check that preview has header information
        assert!(preview[0].contains("Image"));
        assert!(preview[0].contains("test.png"));
        assert!(preview[1].contains("100x100"));
        assert!(preview[2].contains("bytes"));

        // Check that ASCII art was generated (should have more than just the header lines)
        assert!(preview.len() > 4);
    }

    #[test]
    fn test_generate_preview_image_integration() {
        use tempfile::TempDir;

        // Create a temporary directory and a simple image
        let temp_dir = TempDir::new().unwrap();
        let image_path = temp_dir.path().join("photo.jpg");

        // Create a simple 50x50 image
        let img = image::RgbImage::from_fn(50, 50, |_, _| image::Rgb([128, 128, 128]));
        img.save(&image_path).unwrap();

        let file_entry = FileEntry {
            path: image_path.clone(),
            name: "photo.jpg".to_string(),
            size: fs::metadata(&image_path).unwrap().len(),
            modified_date: Utc::now(),
            file_type: FileType::Image,
        };

        let preview = generate_preview(&file_entry).unwrap();

        // Verify that generate_preview dispatches to generate_image_preview
        assert!(preview.len() > 0);
        assert!(preview[0].contains("Image"));
        assert!(preview[1].contains("50x50"));
    }
}
