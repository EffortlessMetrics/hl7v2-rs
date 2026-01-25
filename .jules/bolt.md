## 2025-01-29 - HL7 v2 Escaping Optimization
**Learning:** Adding a "fast path" check (`!text.contains(...)`) for escaping/unescaping functions yielded massive performance gains (82% / 47%) for clean inputs, with only a minor regression (2-14%) for dirty inputs. In HL7 processing, most fields don't contain delimiters, making this trade-off highly favorable.
**Action:** Always check for the necessity of expensive string processing (allocation, char-by-char iteration) before starting it, especially when the "clean" case is the norm.
