## 2025-01-28 - Optimizing String Escaping with str::find
**Learning:** Using `str::find` to locate the first special character allows skipping the 'clean' prefix in string transformations. This avoids allocation for clean strings (huge win) and reduces iteration overhead for dirty strings (small win), avoiding the 'double scanning' regression of a naive `contains` check.
**Action:** Always verify fast paths don't introduce regressions for the slow path. Use `find` + slice indexing instead of just `contains`.
