## 2025-01-29 - Fast Path for String Escaping
**Learning:** Checks like `str::contains` are extremely fast compared to unconditional allocation and iteration, especially for "clean" inputs which are common in data processing. O(n) scan is cheaper than O(n) copy+allocation.
**Action:** Always check if expensive string processing is actually needed before allocating, especially for "clean" dominant cases.
