# Shredator: Secure File & Directory Shredder

[![License: Proprietary](https://img.shields.io/badge/License-Proprietary-red.svg)](https://example.com/license)
[![Rust Version](https://img.shields.io/badge/rust-1.64%2B-blue.svg)](https://www.rust-lang.org/)

Shredator is a powerful, professional-grade command-line utility for securely deleting files and directories. It employs multiple overwrite patterns and industry-standard secure deletion algorithms to ensure your sensitive data cannot be recovered even with specialized forensic tools.

## Table of Contents

- Features
- Installation
- Basic Usage
- Command Line Options
- Shredding Patterns
- Batch Processing
- Examples
- Security Considerations
- Technical Details
- Contributing
- License

## Features

- **Multiple Overwrite Patterns**: Choose from various industry-standard data wiping algorithms
- **Recursive Directory Shredding**: Completely erase entire directory trees
- **Configurable Passes**: Adjust the number of overwrite passes to balance security vs. speed
- **File Filtering**: Include or exclude files based on pattern matching
- **Safety Confirmations**: Confirmation prompts for important files to prevent accidental deletion
- **Batch Processing**: Process multiple files from a list
- **Zero-Name Security**: Option to rename files with random data before deletion
- **Performance Benchmarking**: Measure shredding speed and efficiency
- **Cross-Platform**: Works on Linux, macOS, and Windows

## Installation

### Licensed Users

1. Download the installer package from your licensed user portal
2. Follow the installation instructions for your platform
3. Activate using your license key

For evaluation licenses or purchasing information, please contact: [Your Contact Information]

### From Source

1. Ensure you have Rust and Cargo installed:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/shredator.git
   cd shredator
   ```

3. Build the project:
   ```bash
   cargo build --release
   ```

4. The executable will be available at Shredator

### Using Cargo

```bash
cargo install shredator
```

## Basic Usage

```bash
shredator <file_or_directory_path> [options]
```

Shredator will securely delete the specified file or directory, overwriting it multiple times before deletion to prevent recovery.

## Command Line Options

| Option | Description |
|--------|-------------|
| `-p, --passes <number>` | Number of overwrite passes (default: 3) |
| `-v, --verbose` | Display detailed progress information |
| `-q, --quiet` | Only display errors and final summary |
| `-f, --force` | Skip confirmation prompts for sensitive operations |
| `--pattern <type>` | Overwrite pattern (random, zeros, ones, alt, dod, gutmann) |
| `--max-depth <number>` | Maximum directory depth for recursion |
| `--include <pattern>` | Only process files matching pattern (e.g., '*.txt') |
| `--exclude <pattern>` | Skip files matching pattern (e.g., '*.jpg') |
| `--benchmark` | Measure and report performance statistics |
| `--zero-names` | Rename files to random data before deletion |
| `--file-list <path>` | Read paths to shred from a text file (one path per line) |

## Shredding Patterns

Shredator supports several shredding patterns, each with different security properties:

### Random (Default)
Overwrites the file with random data for the specified number of passes. This is generally secure for most use cases.

### Zeros
Overwrites the file with all zeros (0x00) for each pass.

### Ones
Overwrites the file with all ones (0xFF) for each pass.

### Alternating (`alt`)
Alternates between patterns of 0x55 and 0xAA (alternating bits).

### DoD 5220.22-M (`dod`)
Uses the US Department of Defense standard pattern:
1. Pass 1: All zeros (0x00)
2. Pass 2: All ones (0xFF)
3. Pass 3: Random data
This pattern repeats for the specified number of passes.

### Gutmann (`gutmann`)
Implements Peter Gutmann's 35-pass algorithm designed to thwart even the most sophisticated data recovery attempts. Automatically sets passes to 35.

## Batch Processing

Shredator can process multiple files listed in a text file:

```bash
shredator --file-list paths_to_shred.txt --force
```

Format of the list file:
```
# Comments start with #
/path/to/file1.txt
/path/to/file2.pdf
/path/to/directory
```

Features of batch processing:
- Comments (lines starting with #) are ignored
- Empty lines are skipped
- Reports statistics on successful, failed, and skipped paths
- Continues processing even if some paths fail

## Examples

### Basic File Shredding

```bash
# Shred a single file with default settings (3 passes of random data)
shredator sensitive_document.pdf

# Shred with 7 random passes
shredator financial_data.xlsx --passes 7
```

### Directory Shredding

```bash
# Recursively shred a directory and all its contents
shredator ~/old_projects/confidential/

# Limit recursion depth
shredator ~/logs/ --max-depth 2
```

### Pattern Selection

```bash
# Use DoD standard (3-pass pattern)
shredator ~/tax_returns/ --pattern dod

# Use Gutmann algorithm (35 passes)
shredator ~/crypto_keys/ --pattern gutmann
```

### File Filtering

```bash
# Only shred text files
shredator ~/documents/ --include "*.txt"

# Shred everything except images
shredator ~/documents/ --exclude "*.jpg" --exclude "*.png"
```

### Performance Benchmarking

```bash
# Measure shredding performance
shredator large_file.dat --benchmark
```

### Enhanced Security

```bash
# Rename files to random data before deletion
shredator ~/private/ --zero-names
```

## Security Considerations

### Physical Media Types

- **Hard Disk Drives (HDDs)**: Shredator is highly effective for traditional magnetic drives
- **Solid State Drives (SSDs)**: Due to wear leveling and overprovisioning, secure deletion on SSDs is less guaranteed
- **Flash Media**: Similar limitations apply to USB drives and SD cards

### Effectiveness

Shredator is designed to prevent data recovery by:
1. Overwriting data multiple times with different patterns
2. Truncating files to zero length before deletion
3. Optionally renaming files before deletion

For the highest level of security on SSDs, consider using:
- Full disk encryption
- The manufacturer's secure erase command
- Physical destruction for decommissioned media

## Technical Details

### Buffer Sizes

Shredator uses an 8KB buffer by default for efficient overwriting. This balance provides good performance across various storage media.

### Platform-Specific Syncing

Shredator uses platform-specific syncing methods to ensure data is properly written to disk:
- `sync_all()` on Unix-based systems
- Multiple `flush()` calls on Windows

### Important File Detection

Files are considered "important" (requiring confirmation) if they:
- Have extensions associated with documents, spreadsheets, presentations, or images
- Are larger than 10MB in size

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Feedback and Support

For bug reports, feature requests, or technical support, please contact:

[Your Contact Information]

Selected users may be invited to participate in our closed beta testing program for upcoming features.

---

*Use Shredator responsibly. Always ensure you have proper authorization before deleting data, especially in shared or workplace environments.*
