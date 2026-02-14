# Bolt's Journal

## 2025-05-20 - String Escaping Fast Path
**Learning:** Checking for the presence of special characters before allocating a new string is a massive win for "clean" strings (~85% faster for unescape, ~37% for escape). Using `str::contains` with a slice of chars `&['a', 'b'][..]` is significantly faster than using a closure `|c| c == 'a' || c == 'b'` because it leverages internal optimizations (likely `memchr`).
**Action:** Always check if a transformation is needed before allocating. Use slice-based `contains` for multi-char checks.
