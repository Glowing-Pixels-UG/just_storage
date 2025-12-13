# File Type Detection Reference

## Overview

This document provides a comprehensive reference for file types supported by the smart compression detection system, organized by compression category.

## Already Compressed Formats (Skip Compression)

These formats are already compressed and should **not** be compressed again. Compression would waste CPU time and may expand the file.

### Compressed Image Formats
- **JPEG/JPG**: `image/jpeg`, `image/jpg`
- **PNG**: `image/png`
- **GIF**: `image/gif`
- **WebP**: `image/webp`
- **HEIF/HEIC**: `image/heif`, `image/heic`
- **AVIF**: `image/avif`
- **JPEG XL**: `image/jxl`
- **JPEG 2000**: `image/jp2`, `image/jpx`
- **Photoshop**: `image/vnd.adobe.photoshop`, `image/psd`

### Compressed Archives
- **ZIP**: `application/zip`
- **GZIP**: `application/gzip`, `application/x-gzip`
- **BZIP2**: `application/x-bzip2`
- **7-Zip**: `application/x-7z-compressed`
- **XZ**: `application/x-xz`
- **Zstd**: `application/zstd`
- **LZ4**: `application/x-lz4`
- **LZIP**: `application/x-lzip`
- **RAR**: `application/vnd.rar`, `application/x-rar-compressed`
- **TAR**: `application/x-tar`
- **Compress**: `application/x-compress`
- **BZIP3**: `application/x-bzip3`

### Office Open XML (ZIP-based)
- **DOCX**: `application/vnd.openxmlformats-officedocument.wordprocessingml.document`
- **XLSX**: `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`
- **PPTX**: `application/vnd.openxmlformats-officedocument.presentationml.presentation`
- **Macro-enabled variants**: `.macroenabled.12` variants

### Documents
- **PDF**: `application/pdf` (usually compressed internally)

### E-Books
- **EPUB**: `application/epub+zip` (ZIP-based)

### Compressed Audio Formats
- **MP3**: `audio/mpeg`, `audio/mp3`
- **MP4 Audio**: `audio/mp4`, `audio/x-m4a`
- **AAC**: `audio/aac`
- **OGG**: `audio/ogg`, `audio/vorbis`
- **Opus**: `audio/opus`
- **FLAC**: `audio/x-flac`, `audio/flac`
- **AMR**: `audio/amr`, `audio/amr-wb`
- **APE**: `audio/x-ape`
- **DSF**: `audio/x-dsf`

### Compressed Video Formats
- **MP4**: `video/mp4`
- **M4V**: `video/x-m4v`
- **QuickTime**: `video/quicktime`
- **Matroska**: `video/x-matroska`, `video/mkv`
- **WebM**: `video/webm`
- **AVI**: `video/x-msvideo`, `video/avi`
- **WMV**: `video/x-ms-wmv`
- **MPEG**: `video/mpeg`, `video/mpg`
- **FLV**: `video/x-flv`
- **ASF**: `video/x-ms-asf`
- **3GPP**: `video/3gpp`, `video/3gpp2`

### Font Files
- **WOFF**: `application/font-woff`, `font/woff`
- **WOFF2**: `application/font-woff2`, `font/woff2`
- **TTF/OTF**: `application/font-sfnt`, `application/x-font-ttf`, `application/x-font-otf`

## Highly Compressible Formats (Always Compress)

These formats compress extremely well (often 80-95% reduction) and should **always** be compressed.

### Text Files
- **Plain Text**: `text/plain`
- **HTML**: `text/html`
- **XML**: `text/xml`
- **CSS**: `text/css`
- **JavaScript**: `text/javascript`
- **CSV**: `text/csv`
- **Markdown**: `text/markdown`
- **YAML**: `text/yaml`, `text/x-yaml`
- **Log Files**: `text/x-log`

### Structured Data Formats
- **JSON**: `application/json`
- **XML**: `application/xml`, `text/xml`
- **JavaScript**: `application/javascript`, `application/ecmascript`, `text/javascript`, `text/ecmascript`
- **YAML**: `application/yaml`, `application/x-yaml`, `text/yaml`, `text/x-yaml`
- **TOML**: `application/toml`, `text/toml`, `application/x-toml`

### Source Code Files
- **C/C++**: `text/x-c`, `text/x-c++`, `text/x-c++src`, `text/x-csrc`
- **Java**: `text/x-java`, `text/x-java-source`
- **Python**: `text/x-python`
- **Rust**: `text/x-rust`
- **Go**: `text/x-go`
- **Ruby**: `text/x-ruby`
- **PHP**: `text/x-php`
- **Shell Scripts**: `text/x-shellscript`, `application/x-sh`, `application/x-bash`
- **Perl**: `text/x-perl`
- **Lua**: `text/x-lua`

### Markup and Configuration
- **XHTML**: `application/xhtml+xml`
- **SGML**: `application/sgml`, `text/sgml`
- **INI Files**: `application/x-ini`, `text/x-ini`
- **Config Files**: `application/x-config`, `text/x-properties`

### Data Interchange
- **CSV**: `application/csv`, `text/csv`, `application/x-csv`
- **TSV**: `text/tab-separated-values`, `text/tsv`

## Moderately Compressible Formats (Compress with Fast Algorithm)

These formats can benefit from compression but may not compress as well as text. Use fast compression algorithms.

### Uncompressed Image Formats
- **BMP**: `image/bmp`, `image/x-ms-bmp`, `image/x-bmp`
- **TIFF**: `image/tiff`, `image/tif`, `image/x-tiff`
- **PBM/PGM/PPM**: `image/x-portable-bitmap`, `image/x-portable-pixmap`, `image/x-portable-graymap`, `image/x-portable-anymap`
- **PCX**: `image/x-pcx`
- **PICT**: `image/x-pict`
- **TGA**: `image/x-tga`, `image/x-targa`
- **ICO**: `image/x-icon`, `image/vnd.microsoft.icon`

### Uncompressed Audio Formats
- **WAV**: `audio/wav`, `audio/x-wav`, `audio/wave`, `audio/vnd.wave`, `audio/x-pn-wav`
- **AIFF**: `audio/x-aiff`, `audio/aiff`

### Office Documents (Older Formats)
- **DOC**: `application/msword`, `application/vnd.ms-word`
- **XLS**: `application/vnd.ms-excel`
- **PPT**: `application/vnd.ms-powerpoint`
- **ODT/ODS/ODP**: `application/vnd.oasis.opendocument.*`
- **RTF**: `application/rtf`, `text/rtf`, `application/x-rtf`

### Database Files
- **SQLite**: `application/x-sqlite3`, `application/vnd.sqlite3`
- **SQL**: `application/x-sql`
- **Access**: `application/x-access`, `application/x-msaccess`

## Poorly Compressible Formats (Skip Compression)

These formats are typically incompressible or may expand when compressed.

### Encrypted Files
- **PGP**: `application/pgp-encrypted`, `application/pgp-keys`, `application/pgp-signature`
- **Encrypted**: `application/x-encrypted`

### Executable Files
- **ELF**: `application/x-elf`
- **Mach-O**: `application/x-mach-binary`
- **PE**: `application/vnd.microsoft.portable-executable`, `application/x-msdownload`, `application/x-dosexec`
- **Generic**: `application/x-executable`

### Virtual Machine Disk Images
- **VHD**: `application/x-vhd`
- **VMDK**: `application/x-vmdk`
- **VDI**: `application/x-vdi`
- **QCOW2**: `application/x-qcow2`

## Unknown Formats (Data Analysis)

For formats not in the above lists, the system will:
1. Try magic number detection (if `file-type-detection` feature enabled)
2. Analyze data characteristics:
   - **Entropy**: Low entropy = compressible
   - **Repetition**: High repetition = compressible
   - **Randomness**: High entropy = skip compression

## Detection Priority

1. **MIME Type** (if provided) - Fastest, most accurate
2. **Magic Numbers** (if `file-type-detection` enabled) - Automatic, accurate
3. **Data Analysis** (fallback) - Slower, less accurate

## Magic Number Detection

When the `file-type-detection` feature is enabled, the system uses the `infer` crate to detect file types from magic bytes. This supports 100+ file types including:

- Images: JPEG, PNG, GIF, WebP, HEIF, AVIF, BMP, TIFF, etc.
- Archives: ZIP, GZIP, BZIP2, 7Z, XZ, RAR, TAR, etc.
- Documents: PDF, DOCX, XLSX, PPTX, ODT, etc.
- Audio: MP3, FLAC, WAV, OGG, etc.
- Video: MP4, MKV, WebM, AVI, etc.
- And many more...

See the [infer crate documentation](https://docs.rs/infer/latest/infer/) for the complete list.

## Usage Examples

### With MIME Type (Recommended)

```rust
let mut writer = ContainerWriter::new_smart()
    .set_asset_mime_type("application/pdf");  // Explicit MIME type
writer.add_asset(pdf_data)?;  // Automatically skips compression
```

### Automatic Detection (Magic Numbers)

```rust
let mut writer = ContainerWriter::new_smart();
writer.add_asset(pdf_data)?;  // Detects PDF from magic bytes (%PDF-)
```

### Fallback (Data Analysis)

```rust
let mut writer = ContainerWriter::new_smart();
writer.add_asset(unknown_data)?;  // Analyzes entropy/repetition
```

## Statistics

- **Already Compressed**: ~50+ formats
- **Highly Compressible**: ~30+ formats
- **Moderately Compressible**: ~15+ formats
- **Poorly Compressible**: ~10+ formats
- **Total Supported**: 100+ formats (with magic number detection)

## References

- [IANA Media Types](https://www.iana.org/assignments/media-types/media-types.xhtml)
- [MIME Types Database](https://developer.mozilla.org/en-US/docs/Web/HTTP/Basics_of_HTTP/MIME_types)
- [infer crate](https://docs.rs/infer/latest/infer/) - Magic number detection
- [File Format Compression Characteristics](https://en.wikipedia.org/wiki/List_of_file_formats)

