## 2024-05-21 - CLI Data Visualization
**Learning:** Users struggle to interpret raw byte counts (e.g., "8345710592 bytes") in CLI output. Sorted metrics (alphabetical) are significantly easier to scan than arbitrary hash map order.
**Action:** Always implement a `format_size` helper for file/memory sizes and sort metric keys before display in CLI tools.
