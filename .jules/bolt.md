## 2025-01-28 - Fast Path Optimization for HL7 Escaping
**Learning:** Adding a "fast path" check (`contains`) before processing strings significantly improves performance for "clean" inputs (81% for unescape, 47% for escape) but causes a mild regression for "dirty" inputs (~11-14%) due to double scanning.
**Action:** When optimizing string processing, consider the distribution of input data. For HL7, clean fields dominate, so the trade-off is highly beneficial. Always benchmark both clean and dirty paths to quantify the trade-off.
