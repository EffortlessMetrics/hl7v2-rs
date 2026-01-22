## 2026-01-22 - Consistent Metric Ordering
**Learning:** Randomly ordered metrics in CLI output (due to HashMap iteration) increase cognitive load and make comparison difficult; sorting them alphabetically makes the output skimmable and consistent.
**Action:** Always sort map/hashmap keys before displaying them in user-facing output.
