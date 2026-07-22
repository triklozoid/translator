# Ideas for Further Improvement

## Stream Stalling (current focus)
- Add a "Retry" button in the UI for stalled/failed translations
- Implement TCP keepalive on the reqwest client (SO_KEEPALIVE) to detect silent drops faster than OS defaults
- Add configurable timeout values in config.toml (stall_timeout, stall_deadline)
- Pre-flight health check: ping the API endpoint before starting translation

## Diagnostics
- Add an in-app log viewer (or a button to open ~/.config/translator/app.log)
- Structured logging: JSON format for machine-parseable diagnostics
- Per-provider latency metrics to auto-tune timeouts

## UX
- Progress spinner / animated dots during active translation
- Show token count / translation speed in status bar
- Dark/light theme support via GTK CSS

## Robustness
- Persistent translation cache to avoid re-translating identical text
- Offline language detection fallback (already using lingua, but could cache results)
- Graceful degradation: if streaming fails, fall back to non-streaming API call
