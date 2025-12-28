const DEFAULT_TYPE: &str = "application/octet-stream";

pub fn media_type_from_path(_file_path: &str) -> &'static str {
    if let Some((_, ext)) = _file_path.rsplit_once(".") {
        match ext {
            // Web Stuff
            "html" => "text/html",
            "htm" => "text/html",
            "css" => "text/css",
            "js" => "text/javascript",
            "mjs" => "text/javascript",
            "json" => "application/json",
            "xhtml" => "application/xhtml+xml",
            "xml" => "application/xml",
            "webmanifest" => "application/manifest+json",

            // Documents
            "md" => "text/markdown",
            "pdf" => "application/pdf",
            "txt" => "text/plain",

            // Images
            "avif" => "image/avif",
            "gif" => "image/gif",
            "ico" => "image/vnd.microsoft.icon",
            "jpeg" => "image/jpeg",
            "jpg" => "image/jpeg",
            "png" => "image/png",
            "svg" => "image/svg+xml",
            "webp" => "image/webp",
            "heif" => "image/heif",
            "heic" => "image/heic",
            "jxl" => "image/jxl",

            // Audio
            "wav" => "audio/wav",
            "weba" => "audio/webm",
            "mp3" => "audio/mpeg",
            "oga" => "audio/ogg",
            "opus" => "audio/ogg",

            // Video & Media Container
            "mp4" => "video/mp4",
            "mpeg" => "video/mpeg",
            "ogv" => "video/ogg",
            "webm" => "video/webm",
            "mkv" => "video/x-matroska",
            "ogx" => "application/ogg",

            // Fonts
            "ttf" => "font/ttf",
            "woff" => "font/woff",
            "woff2" => "font/woff2",

            // Fallback
            _ => DEFAULT_TYPE,
        }
    } else {
        DEFAULT_TYPE
    }
}
