## 2024-05-21 - CLI Output Readability
**Learning:** Raw byte counts in CLI output (e.g., "12345678 bytes") are difficult for users to interpret quickly.
**Action:** Implement a reusable `format_size` helper for any user-facing metrics to display human-readable units (KB, MB, GB).
