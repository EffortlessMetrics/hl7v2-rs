## 2023-11-20 - API Root Landing Page
**Learning:** For headless services, the root endpoint ('/') acts as the UI. Returning a JSON 'welcome' message with service info and links prevents 404s and aids discoverability.
**Action:** Always verify if a backend service has a root handler; if not, add one serving basic metadata.
