## 2023-11-20 - API Key Security
**Vulnerability:** Timing attack potential in API key verification and insecure default configuration risk.
**Learning:** String equality checks (`==`) verify characters sequentially, allowing timing side-channel attacks. Also, falling back to a default 'test' key when configuration is missing violates 'Fail Secure' principles.
**Prevention:** Use constant-time comparison algorithms (e.g., XOR accumulation) for secrets. Ensure the application fails to start if critical secrets are missing rather than using weak defaults.
