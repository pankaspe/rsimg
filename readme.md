# 🦀 RS img

**Rust-powered parallel image optimizer.** Resize and convert images to multiple formats with real-time progress visualization.

![RS img](./screenshot)

## ✨ Features

- 🚀 Parallel processing with Rayon
- 📦 Multi-format: JPG, WebP, PNG
- 🎯 Multiple scales in one pass
- 📁 Recursive directory processing
- 💾 Custom output directory
- 🎨 Real-time progress bars for each image

## 📦 Installation

```bash
# Clone and build
git clone https://github.com/pankaspe/rsimg.git
cd rsimg
cargo build --release

# Or install locally
cargo install --path .
```

## 🚀 Usage

### Basic Syntax
```bash
rsimg [OPTIONS] <INPUT>
```

### Quick Start
```bash
# Optimize a single image (saves to same folder)
rsimg photo.jpg

# Optimize all images in a folder
rsimg ./images

# Recursive with custom output
rsimg ./photos --output ./optimized --recursive
```

## ⚙️ Options

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--formats` | | Output formats (comma-separated) | `jpg,webp` |
| `--scales` | | Scale percentages (comma-separated) | `75,50,25` |
| `--quality` | | Compression quality (0-100) | `80` |
| `--output` | `-o` | Output directory | same as input |
| `--recursive` | `-r` | Process subdirectories | `false` |
| `--threads` | `-t` | Number of threads | auto |

### Examples

```bash
# Web-ready images with custom quality
rsimg ./photos --formats webp,jpg --scales 100,75,50 --quality 85

# Organize output separately
rsimg ./raw --output ./web_ready --recursive

# Create thumbnails only
rsimg ./gallery --scales 25,50 --quality 70

# Limit CPU usage on laptop
rsimg ./images --threads 2 --recursive

# Convert to WebP only at original size
rsimg ./pngs --formats webp --scales 100 --quality 90
```

## 📊 Output Example

```
=== rsimg — Image Optimizer ===

📁 Found 8 images
💾 Output: ./optimized
⚙️  Formats: webp, jpg | Scales: 75%, 50%, 25% | Quality: 85%

📄 beach-sunset.jpg                      [━━━━━━━━━━━━━━╾─────────────────────────]  4/6
📄 mountain-view.jpg                     [━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━]  6/6
✓ ocean-waves.jpg                        [━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━]  6/6
📄 city-night.jpg                        [━━━━━━━━━╾──────────────────────────────]  2/6

✓ Processing completed successfully!
   8 images optimized
```

## 💾 Output Behavior

**Without `--output`** (default):
```
photos/
├── sunset.jpg              # original
├── sunset_75pct.jpg        # generated
├── sunset_75pct.webp       # generated
└── sunset_50pct.jpg        # generated
```

**With `--output ./optimized`**:
```
photos/
└── sunset.jpg              # original untouched

optimized/                  # created automatically
├── sunset_75pct.jpg
├── sunset_75pct.webp
└── sunset_50pct.jpg
```

## 🎯 Supported Formats

**Input**: JPG, PNG, WebP, GIF, BMP, TIFF, ICO  
**Output**: JPG, WebP, PNG

## 💡 Quality Guide

- **90-95**: Very high (print/archive)
- **80-85**: High (web standard) ⭐ **Recommended**
- **70-75**: Good (balanced)
- **60-65**: Acceptable (maximum compression)

## 📄 License

MIT License
