## 2024-03-24 - String Processing Fast Path
**Learning:** For string transformation functions (like escaping/unescaping) where the transformation is only needed for a subset of inputs (e.g. those with special chars), checking for the presence of those characters before allocating and processing yields massive performance gains (70%+) for the "clean" case.
**Action:** Always check `str::contains` or similar lightweight checks before entering allocation-heavy loops for string processing.
