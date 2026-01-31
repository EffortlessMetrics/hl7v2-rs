## 2025-05-27 - Fast Path for String Processing
**Learning:** Checking for the presence of delimiters before processing strings can yield massive performance gains (over 80%!) for "clean" inputs, which are often the common case.
**Action:** Always consider adding a "fast path" that returns the original string (or a Cow) if no modification is needed, especially when the modification involves allocation and iteration. The cost of `str::contains` is negligible compared to the savings.
