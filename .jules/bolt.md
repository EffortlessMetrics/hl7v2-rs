## 2025-05-28 - Optimizing HL7 Escaping for Clean Text
**Learning:** In HL7 v2 processing, the vast majority of text fields do not contain characters that require escaping. Optimizing `escape_text` and `unescape_text` with a "fast path" that checks for special characters before allocating significantly improves performance.
**Action:** Always verify if a transformation is actually needed before performing it. For string processing, `str::contains` (even with multiple chars) is much faster than iterating and building a new string.
