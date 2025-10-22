# noway

A CLI tool to download archived pages from the Wayback Machine.

## Installation

```bash
cargo install noway
```

Or build from source:

```bash
cargo build --release
```

## Usage

Download all archived versions of a URL:

```bash
noway example.com
```

### Options

- `-o, --output <DIR>` - Specify output directory (default: random name)
- `-m, --match-type <TYPE>` - URL match type (default: `prefix`)
- `-c, --concurrency <N>` - Max concurrent downloads (default: `5`)

### Examples

Download with custom output directory:

```bash
noway example.com -o my_archive
```

Download with higher concurrency:

```bash
noway example.com -c 10
```

## License

See LICENSE file for details.
