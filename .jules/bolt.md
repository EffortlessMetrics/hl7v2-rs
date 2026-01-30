## 2025-05-23 - Fast Path Optimization for Text Escaping
**Learning:** Checking for special characters before allocating/processing yields massive gains for "clean" text (~80% faster for unescape, ~20% for escape) even with the overhead of scanning the string. The regression for "dirty" text is minimal (~4%).
**Action:** Always consider a "fast path" for the common case (clean data) when the processing involves allocation or complex logic. Use `str::contains` which is highly optimized.
