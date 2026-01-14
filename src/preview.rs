// Preview module for generating file previews with syntax highlighting, images, and PDFs
#![allow(dead_code)]

use crate::domain::FileEntry;
use image::{DynamicImage, GenericImageView, Pixel};
use pdfium_render::prelude::*;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::fs;
use std::io;
use std::path::Path;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

const MAX_PREVIEW_LINES: usize = 50;
const MAX_IMAGE_WIDTH: u32 = 160;
/// Height is halved because we render 2 pixels per terminal row using half-blocks
const MAX_IMAGE_HEIGHT: u32 = 100;

/// Represents preview content that can be either plain text or styled image lines
#[derive(Debug, Clone)]
pub enum PreviewContent {
    /// Plain text lines (for text files, binary info, etc.)
    Text(Vec<String>),
    /// Styled lines with color information (for images and PDFs)
    Styled(Vec<Line<'static>>),
}

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

/// Converts an image to styled lines using half-block characters for terminal display.
/// Uses the upper half block character (▀) with foreground color for the upper pixel
/// and background color for the lower pixel, effectively displaying 2 pixels per cell.
pub fn image_to_halfblock_lines(img: &DynamicImage, width: u32, height: u32) -> Vec<Line<'static>> {
    // Ensure height is even for proper half-block rendering
    let height = if height.is_multiple_of(2) {
        height
    } else {
        height + 1
    };

    // Use Triangle filter - fast and good quality for terminal display
    // (Lanczos3 is too slow for large images like 4K wallpapers)
    let img = img.resize_exact(width, height, image::imageops::FilterType::Triangle);
    let img = img.to_rgb8();

    let term_height = height / 2;
    let mut lines = Vec::with_capacity(term_height as usize);

    for y in 0..term_height {
        let upper_y = y * 2;
        let lower_y = upper_y + 1;

        let mut spans = Vec::with_capacity(width as usize);

        for x in 0..width {
            let upper_pixel = img.get_pixel(x, upper_y).to_rgb();
            let lower_pixel = if lower_y < height {
                img.get_pixel(x, lower_y).to_rgb()
            } else {
                upper_pixel
            };

            // Upper half block: ▀ (U+2580)
            // Foreground color = upper pixel, Background color = lower pixel
            let style = Style::default()
                .fg(Color::Rgb(upper_pixel[0], upper_pixel[1], upper_pixel[2]))
                .bg(Color::Rgb(lower_pixel[0], lower_pixel[1], lower_pixel[2]));

            spans.push(Span::styled("▀", style));
        }

        lines.push(Line::from(spans));
    }

    lines
}

/// Generates an image preview using half-block character rendering for true color display
pub fn generate_image_preview(file_entry: &FileEntry) -> io::Result<PreviewContent> {
    let img = load_image(&file_entry.path)?;
    let (original_width, original_height) = img.dimensions();

    // Calculate resize dimensions - note that the height will be halved in rendering
    // because we use 2 pixels per terminal row
    let (new_width, new_height) = calculate_resize_dimensions(
        original_width,
        original_height,
        MAX_IMAGE_WIDTH,
        MAX_IMAGE_HEIGHT,
    );

    // Create header lines with image info
    let header_style = Style::default().add_modifier(Modifier::BOLD);
    let info_style = Style::default().fg(Color::Gray);

    let mut lines: Vec<Line<'static>> = vec![
        Line::from(vec![
            Span::styled("Image: ", header_style),
            Span::styled(file_entry.name.clone(), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled(
                format!("Dimensions: {}×{} px", original_width, original_height),
                info_style,
            ),
            Span::raw("  "),
            Span::styled(format!("Size: {} bytes", file_entry.size), info_style),
        ]),
        Line::from(""),
    ];

    // Generate half-block image lines
    let image_lines = image_to_halfblock_lines(&img, new_width, new_height);
    lines.extend(image_lines);

    Ok(PreviewContent::Styled(lines))
}

/// Attempts to create a Pdfium instance using explicit binding (no panic)
fn try_create_pdfium() -> Option<Pdfium> {
    // Try multiple locations for the Pdfium library:
    // 1. System library paths (standard locations like /usr/local/lib)
    // 2. PDFIUM_DYNAMIC_LIB_PATH environment variable
    // 3. lib/ subdirectory (for bundled distributions)
    // 4. Current directory

    // Try system library first
    if let Ok(bindings) = Pdfium::bind_to_system_library() {
        return Some(Pdfium::new(bindings));
    }

    // Try environment variable path
    if let Ok(lib_path) = std::env::var("PDFIUM_DYNAMIC_LIB_PATH") {
        if let Ok(bindings) =
            Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(&lib_path))
        {
            return Some(Pdfium::new(bindings));
        }
    }

    // Try lib/ subdirectory
    if let Ok(bindings) =
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./lib/"))
    {
        return Some(Pdfium::new(bindings));
    }

    // Try current directory
    if let Ok(bindings) =
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
    {
        return Some(Pdfium::new(bindings));
    }

    None
}

/// Checks if Pdfium library is available by attempting to initialize it
pub fn is_pdfium_available() -> bool {
    try_create_pdfium().is_some()
}

/// Loads a PDF and renders the first page to an image
pub fn render_pdf_first_page(path: &Path) -> io::Result<DynamicImage> {
    // Initialize Pdfium library using explicit binding (no panic)
    let pdfium = try_create_pdfium().ok_or_else(|| {
        io::Error::other("Pdfium library not available. Install libpdfium to enable PDF previews.")
    })?;

    // Load the PDF document
    let document = pdfium
        .load_pdf_from_file(path, None)
        .map_err(|e| io::Error::other(format!("PDF loading error: {}", e)))?;

    // Get the first page
    let page = document
        .pages()
        .get(0)
        .map_err(|e| io::Error::other(format!("PDF page access error: {}", e)))?;

    // Render the page to a bitmap at 72 DPI (standard screen resolution)
    let render_config = PdfRenderConfig::new()
        .set_target_width(1024)
        .set_maximum_height(1024);

    let bitmap = page
        .render_with_config(&render_config)
        .map_err(|e| io::Error::other(format!("PDF rendering error: {}", e)))?;

    // Convert bitmap to image::DynamicImage
    let width = bitmap.width() as u32;
    let height = bitmap.height() as u32;

    // Get the raw bytes and stride information
    // The stride may include padding bytes for memory alignment
    let raw_buffer = bitmap.as_raw_bytes();
    let stride = raw_buffer.len() / (height as usize);
    let expected_row_bytes = (width as usize) * 4; // 4 bytes per pixel (RGBA)

    // Handle stride padding: if stride > expected row bytes, we need to copy row by row
    let buffer = if stride > expected_row_bytes {
        // Create a new buffer without stride padding
        let mut clean_buffer = Vec::with_capacity((width as usize) * (height as usize) * 4);
        for row in 0..height as usize {
            let row_start = row * stride;
            let row_end = row_start + expected_row_bytes;
            if row_end <= raw_buffer.len() {
                clean_buffer.extend_from_slice(&raw_buffer[row_start..row_end]);
            }
        }
        clean_buffer
    } else {
        raw_buffer.to_vec()
    };

    // Create an RGBA image from the buffer
    let img_buffer = image::RgbaImage::from_vec(width, height, buffer).ok_or_else(|| {
        io::Error::other(format!(
            "Failed to create image from PDF bitmap: buffer size {} doesn't match {}x{}x4={}",
            raw_buffer.len(),
            width,
            height,
            width * height * 4
        ))
    })?;

    Ok(DynamicImage::ImageRgba8(img_buffer))
}

/// Extracts text content from a PDF file
fn extract_pdf_text(path: &Path, max_lines: usize) -> io::Result<Vec<String>> {
    let pdfium = try_create_pdfium().ok_or_else(|| {
        io::Error::other("Pdfium library not available. Install libpdfium to enable PDF previews.")
    })?;

    let document = pdfium
        .load_pdf_from_file(path, None)
        .map_err(|e| io::Error::other(format!("PDF loading error: {}", e)))?;

    let mut all_text = String::new();
    let page_count = document.pages().len();

    // Extract text from pages until we have enough content
    for page_index in 0..page_count.min(5) {
        // Limit to first 5 pages
        if let Ok(page) = document.pages().get(page_index) {
            if let Ok(text_page) = page.text() {
                let page_text = text_page.all();
                if !page_text.is_empty() {
                    if !all_text.is_empty() {
                        all_text.push_str("\n\n--- Page ");
                        all_text.push_str(&(page_index + 1).to_string());
                        all_text.push_str(" ---\n\n");
                    }
                    all_text.push_str(&page_text);
                }
            }
        }

        // Stop if we have enough text
        if all_text.lines().count() >= max_lines {
            break;
        }
    }

    // Split into lines and limit
    let lines: Vec<String> = all_text
        .lines()
        .take(max_lines)
        .map(|s| s.to_string())
        .collect();

    Ok(lines)
}

/// Generates a PDF preview by extracting text content
pub fn generate_pdf_preview(file_entry: &FileEntry) -> io::Result<PreviewContent> {
    // Try to extract text from the PDF
    match extract_pdf_text(&file_entry.path, MAX_PREVIEW_LINES) {
        Ok(text_lines) => {
            let mut lines = vec![
                format!("PDF: {}", file_entry.name),
                format!("Size: {} bytes", file_entry.size),
                String::new(),
            ];

            if text_lines.is_empty() {
                lines.push(
                    "[This PDF contains no extractable text (may be scanned/image-based)]"
                        .to_string(),
                );
                lines.push(String::new());
                lines.push("Press 'o' to open in your default PDF viewer.".to_string());
            } else {
                lines.extend(text_lines);
            }

            Ok(PreviewContent::Text(lines))
        }
        Err(e) => {
            // If PDF text extraction fails, return error information
            let error_msg = e.to_string();
            let help_msg = if error_msg.contains("Pdfium library not available") {
                "[PDF preview requires the Pdfium library. See: https://pdfium.googlesource.com/pdfium/]"
            } else {
                "[This PDF may be corrupted, password-protected, or use unsupported features]"
            };

            Ok(PreviewContent::Text(vec![
                format!("PDF: {}", file_entry.name),
                format!("Size: {} bytes", file_entry.size),
                String::new(),
                format!("Error: {}", error_msg),
                String::new(),
                help_msg.to_string(),
                String::new(),
                "Press 'o' to open in your default PDF viewer.".to_string(),
            ]))
        }
    }
}

/// Generates a preview for any file type
pub fn generate_preview(file_entry: &FileEntry) -> io::Result<PreviewContent> {
    use crate::domain::FileType;

    match file_entry.file_type {
        FileType::Text => generate_text_preview(file_entry).map(PreviewContent::Text),
        FileType::Binary => Ok(PreviewContent::Text(vec![
            format!("Binary file: {}", file_entry.name),
            format!("Size: {} bytes", file_entry.size),
            String::new(),
            "[Binary content not displayed]".to_string(),
        ])),
        FileType::Image => generate_image_preview(file_entry),
        FileType::Pdf => generate_pdf_preview(file_entry),
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
        match preview {
            PreviewContent::Text(lines) => {
                assert!(!lines.is_empty());
                assert!(lines[0].contains("Binary file"));
                assert!(lines.iter().any(|line| line.contains("not displayed")));
            }
            _ => panic!("Expected Text preview for binary file"),
        }
    }

    #[test]
    fn test_generate_preview_pdf_dispatches_correctly() {
        // Test that generate_preview correctly dispatches PDF files to generate_pdf_preview
        // Uses a non-existent file which will result in an error preview
        let file_entry = FileEntry {
            path: PathBuf::from("/nonexistent/test.pdf"),
            name: "test.pdf".to_string(),
            size: 4096,
            modified_date: Utc::now(),
            file_type: FileType::Pdf,
        };

        let preview = generate_preview(&file_entry).unwrap();
        match preview {
            PreviewContent::Text(lines) => {
                assert!(!lines.is_empty());
                // Should contain PDF reference in header
                assert!(lines[0].contains("PDF"));
                // Non-existent file will show error message about rendering
                let preview_text = lines.join(" ");
                assert!(
                    preview_text.contains("Error")
                        || preview_text.contains("corrupted")
                        || preview_text.contains("not available"),
                    "Expected error message for non-existent PDF: {}",
                    preview_text
                );
            }
            PreviewContent::Styled(_) => {
                panic!("Expected Text preview for non-existent PDF (error case)");
            }
        }
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
    fn test_image_to_halfblock_dimensions() {
        // Create a simple 10x10 image
        let img = DynamicImage::new_rgb8(10, 10);
        // Request 5 width and 6 height (even for proper half-block rendering)
        let lines = image_to_halfblock_lines(&img, 5, 6);

        // Height 6 produces 3 terminal rows (6/2)
        assert_eq!(lines.len(), 3);
        // Each line should have 5 spans (one per column)
        for line in &lines {
            assert_eq!(line.spans.len(), 5);
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

        match preview {
            PreviewContent::Styled(lines) => {
                // Should have header lines plus image content
                assert!(lines.len() > 4);
                // First line should contain "Image:" (in a Span)
                let first_line_text: String = lines[0]
                    .spans
                    .iter()
                    .map(|s| s.content.to_string())
                    .collect();
                assert!(first_line_text.contains("Image"));
                assert!(first_line_text.contains("test.png"));
            }
            _ => panic!("Expected Styled preview for image"),
        }
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
        match preview {
            PreviewContent::Styled(lines) => {
                assert!(!lines.is_empty());
                let first_line_text: String = lines[0]
                    .spans
                    .iter()
                    .map(|s| s.content.to_string())
                    .collect();
                assert!(first_line_text.contains("Image"));
            }
            _ => panic!("Expected Styled preview for image"),
        }
    }

    // PDF preview tests
    #[test]
    fn test_render_pdf_first_page_nonexistent() {
        // Skip test if Pdfium library is not available
        if !is_pdfium_available() {
            eprintln!("Skipping PDF test: Pdfium library not available");
            return;
        }

        let result = render_pdf_first_page(Path::new("/nonexistent/file.pdf"));
        assert!(result.is_err());
    }

    #[test]
    fn test_render_pdf_first_page_with_real_pdf() {
        use printpdf::*;
        use tempfile::TempDir;

        // Skip test if Pdfium library is not available
        if !is_pdfium_available() {
            eprintln!("Skipping PDF test: Pdfium library not available");
            return;
        }

        // Create a temporary directory and a simple PDF
        let temp_dir = TempDir::new().unwrap();
        let pdf_path = temp_dir.path().join("test.pdf");

        // Create a simple PDF with one page
        let (doc, _page1, _layer1) = PdfDocument::new("Test PDF", Mm(210.0), Mm(297.0), "Layer 1");

        // Save the PDF
        doc.save(&mut std::io::BufWriter::new(
            std::fs::File::create(&pdf_path).unwrap(),
        ))
        .unwrap();

        // Test rendering the PDF
        let result = render_pdf_first_page(&pdf_path);
        assert!(result.is_ok());

        let img = result.unwrap();
        let (width, height) = img.dimensions();
        assert!(width > 0);
        assert!(height > 0);
    }

    #[test]
    fn test_generate_pdf_preview_with_real_pdf() {
        use printpdf::*;
        use tempfile::TempDir;

        // Skip test if Pdfium library is not available
        if !is_pdfium_available() {
            eprintln!("Skipping PDF test: Pdfium library not available");
            return;
        }

        // Create a temporary directory and a simple PDF
        let temp_dir = TempDir::new().unwrap();
        let pdf_path = temp_dir.path().join("document.pdf");

        // Create a simple PDF with one page
        let (doc, _page1, _layer1) =
            PdfDocument::new("Test Document", Mm(210.0), Mm(297.0), "Layer 1");

        // Save the PDF
        doc.save(&mut std::io::BufWriter::new(
            std::fs::File::create(&pdf_path).unwrap(),
        ))
        .unwrap();

        let file_entry = FileEntry {
            path: pdf_path.clone(),
            name: "document.pdf".to_string(),
            size: fs::metadata(&pdf_path).unwrap().len(),
            modified_date: Utc::now(),
            file_type: FileType::Pdf,
        };

        let preview = generate_pdf_preview(&file_entry).unwrap();

        match preview {
            PreviewContent::Text(lines) => {
                // Should have header lines (PDF name, size, empty line)
                assert!(lines.len() >= 3);
                assert!(lines[0].contains("PDF"));
                assert!(lines[0].contains("document.pdf"));
            }
            PreviewContent::Styled(_) => {
                panic!("Expected Text preview for PDF");
            }
        }
    }

    #[test]
    fn test_generate_pdf_preview_nonexistent() {
        let file_entry = FileEntry {
            path: PathBuf::from("/nonexistent/file.pdf"),
            name: "file.pdf".to_string(),
            size: 0,
            modified_date: Utc::now(),
            file_type: FileType::Pdf,
        };

        let preview = generate_pdf_preview(&file_entry).unwrap();

        // Should return Text variant with error message
        match preview {
            PreviewContent::Text(lines) => {
                assert!(!lines.is_empty());
                assert!(lines[0].contains("PDF"));
                let preview_text = lines.join(" ");
                assert!(
                    preview_text.contains("Error") || preview_text.contains("corrupted"),
                    "Expected error message in preview"
                );
            }
            _ => panic!("Expected Text preview for non-existent PDF"),
        }
    }

    #[test]
    fn test_generate_pdf_preview_invalid_pdf() {
        use tempfile::TempDir;

        // Create a temporary directory and an invalid "PDF" file
        let temp_dir = TempDir::new().unwrap();
        let pdf_path = temp_dir.path().join("invalid.pdf");

        // Write invalid content (not a real PDF)
        fs::write(&pdf_path, b"This is not a valid PDF file").unwrap();

        let file_entry = FileEntry {
            path: pdf_path.clone(),
            name: "invalid.pdf".to_string(),
            size: fs::metadata(&pdf_path).unwrap().len(),
            modified_date: Utc::now(),
            file_type: FileType::Pdf,
        };

        let preview = generate_pdf_preview(&file_entry).unwrap();

        // Should return Text variant with error message
        match preview {
            PreviewContent::Text(lines) => {
                assert!(!lines.is_empty());
                let preview_text = lines.join(" ");
                assert!(
                    preview_text.contains("Error") || preview_text.contains("corrupted"),
                    "Expected error message for invalid PDF"
                );
            }
            _ => panic!("Expected Text preview for invalid PDF"),
        }
    }

    #[test]
    fn test_generate_preview_pdf_integration() {
        use printpdf::*;
        use tempfile::TempDir;

        // Skip test if Pdfium library is not available
        if !is_pdfium_available() {
            eprintln!("Skipping PDF test: Pdfium library not available");
            return;
        }

        // Create a temporary directory and a simple PDF
        let temp_dir = TempDir::new().unwrap();
        let pdf_path = temp_dir.path().join("report.pdf");

        // Create a simple PDF
        let (doc, _page1, _layer1) = PdfDocument::new("Report", Mm(210.0), Mm(297.0), "Layer 1");

        doc.save(&mut std::io::BufWriter::new(
            std::fs::File::create(&pdf_path).unwrap(),
        ))
        .unwrap();

        let file_entry = FileEntry {
            path: pdf_path.clone(),
            name: "report.pdf".to_string(),
            size: fs::metadata(&pdf_path).unwrap().len(),
            modified_date: Utc::now(),
            file_type: FileType::Pdf,
        };

        let preview = generate_preview(&file_entry).unwrap();

        // Verify that generate_preview dispatches to generate_pdf_preview
        match preview {
            PreviewContent::Text(lines) => {
                assert!(!lines.is_empty());
                assert!(lines[0].contains("PDF"));
            }
            _ => panic!("Expected Text preview for PDF"),
        }
    }
}
