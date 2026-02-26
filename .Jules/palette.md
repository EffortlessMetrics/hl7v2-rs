## 2025-05-18 - CLI UX Enhancements
**Learning:** Users often struggle with raw byte counts in CLI output, making it hard to gauge scale at a glance. Additionally, long lists of commands in `--help` or interactive modes can be overwhelming without logical grouping.
**Action:** Always implement a `format_size` utility for any CLI tool dealing with files or memory. Group commands by category (Processing, Validation, etc.) in help output to improve scannability and discoverability.
