# Stream Stall Fix: Final Analysis

## Actual Root Cause (determined from app.log analysis)

**The streaming and API layers work correctly.** The log shows:
- 413 SSE chunks received from API
- 2837 characters translated successfully in 6 seconds
- Stream ended normally: `[Stream] ended normally: 413 chunks total`

The problem is purely in **UI rendering**, not in data delivery.

### What was happening

```
Window (900×800px)
└── Label (wrap=true, vexpand=true, valign=START)
    └── 2837 chars of wrapped text → ~45 lines → ~850px tall
```

The `Label` widget was placed directly inside a `GtkBox` without a `ScrolledWindow`.
When translation text exceeded the window height (~800px):

1. New text was added **below the visible viewport**
2. The user saw the top of the translation freeze
3. GTK continued receiving `set_text()` calls but the visible area didn't change
4. User perceived this as "translation stopped mid-way"

Additionally, `set_text()` was called on **every single chunk** (~60+ times/second),
causing GTK layout thrashing with increasingly large strings (up to 2837 chars).

### Fixes applied

1. **Wrapped Label in ScrolledWindow** (`ui.rs`): Long text is now scrollable,
   content doesn't overflow the window

2. **Auto-scroll to bottom** (`translation.rs`): After each `set_text()`, the
   ScrolledWindow scrolls to the bottom so the user always sees the latest text

3. **UI update throttling** (`translation.rs`): Label updates are limited to
   every ~50ms during rapid streaming, reducing GTK layout pressure

4. **Final flush** (`translation.rs`): After the receive loop exits, a final
   `set_text()` ensures the complete translation is displayed even if the last
   chunks were throttled

### Files changed

| File | Change |
|------|--------|
| `src/ui.rs` | Added `ScrolledWindow` wrapping the translation label |
| `src/translation.rs` | Added auto-scroll, 50ms throttle, final flush after loop |

### Files from previous iteration (stream layer fixes — kept as safety net)

| File | Change |
|------|--------|
| `lib/llm-connector/src/sse.rs` | Buffer flush on stream end, SSE diagnostic logging |
| `src/translation.rs` | Stall timeout (60s), heartbeat with hard deadline (90s) |
