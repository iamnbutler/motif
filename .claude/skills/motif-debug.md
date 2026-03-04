---
name: motif-debug
description: Use when debugging a running motif application - inspecting scene state, quads, text runs, input state, taking screenshots, or drawing debug overlays
---

# motif-debug CLI

Debug and inspect running motif applications via Unix socket IPC.

## When to Use

- Inspecting what's rendered (quads, text runs, scene stats)
- Checking input state (cursor position, pressed buttons, modifiers)
- Taking screenshots of the running app
- Drawing debug overlays to visualize layout or hit regions

## Quick Reference

```bash
# Single command (use --json for machine-readable output)
motif-debug scene.stats
motif-debug --json scene.quads

# REPL mode
motif-debug
motif> scene.stats
motif> quit

# Specific socket (if multiple motif apps running)
motif-debug --socket /tmp/motif-debug-12345.sock scene.stats
```

## Commands

### Scene Inspection

| Command | Description |
|---------|-------------|
| `scene.stats` | Quad count, text run count, viewport size, scale factor |
| `scene.quads` | List all quads with bounds, color, border, corner radii |
| `scene.text_runs` | List all text runs with origin, font size, glyph count |

### Input Inspection

| Command | Description |
|---------|-------------|
| `input.state` | Current cursor position, pressed buttons, modifier keys |

### Screenshots

```bash
screenshot /path/to/file.png    # save to specific path
screenshot                       # auto: /tmp/motif-screenshot-{timestamp}.png
```

### Debug Overlays

Draw colored rectangles over the scene (persist until cleared):

```bash
draw.quad x y w h r g b a       # coords in logical pixels, color 0.0-1.0
draw.quad 100 100 200 50 1 0 0 0.5   # red semi-transparent box
```

Manage overlays:

```bash
debug.list          # list all overlays with IDs
debug.remove <id>   # remove specific overlay
debug.clear         # remove all overlays
```

## Socket Discovery

Servers listen at `/tmp/motif-debug-{pid}.sock`. The CLI auto-discovers running servers. If multiple apps are running, use `--socket` to specify which one.

## Usage Patterns

**Verify rendering:**
```bash
motif-debug --json scene.stats | jq '.quad_count'
```

**Debug hit testing:**
```bash
# Draw overlay at suspected hit region
motif-debug "draw.quad 100 100 50 50 0 1 0 0.3"
```

**Capture state for bug report:**
```bash
motif-debug screenshot /tmp/bug.png
motif-debug --json scene.quads > /tmp/quads.json
```
