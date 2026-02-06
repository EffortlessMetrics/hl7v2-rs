## 2025-01-29 - HL7 String Escaping Optimization
**Learning:** `str::contains` with a closure is significantly slower than with a character slice pattern for multiple delimiters in Rust.
**Action:** When checking for multiple characters, prefer `text.contains(&['a', 'b'][..])` over `text.contains(|c| c == 'a' || c == 'b')`.
