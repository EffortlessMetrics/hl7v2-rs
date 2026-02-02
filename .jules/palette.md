# Palette's Journal

## 2025-01-28 - Human-Readable CLI Output
**Learning:** Users find raw byte counts (e.g., 8124512 bytes) difficult to parse at a glance. Converting to human-readable units (KB, MB, GB) significantly reduces cognitive load.
**Action:** Always implement a `format_size` helper for any CLI command that outputs file or memory sizes. Prioritize custom helpers over dependencies to keep the binary small.

## 2025-01-28 - Deterministic Metric Sorting
**Learning:** Performance metrics stored in HashMaps output in random order, making it hard for users to compare runs or find specific metrics.
**Action:** Always sort map-based metrics alphabetically (or by value, if more appropriate) before displaying them to the user.
