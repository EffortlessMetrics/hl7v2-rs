## 2025-05-21 - Allocation Avoidance in Text Processing
**Learning:** For text transformation tasks (like escaping/unescaping), the "happy path" often requires no changes. Checking for the presence of special characters *before* allocating a result buffer can yield massive performance gains (82-88% in this case) by avoiding allocation entirely for clean input.
**Action:** Always implement a fast-path check using `str::contains(&[...])` or similar to return Cow::Borrowed or the original reference when no modification is needed.
