# Repo Assist Memory — iamnbutler/motif

## Last Updated
2026-09-15

## Repository State
- Main branch: `57a5ad8` (feat(ci): add release workflow and version bump helper) — unchanged since March 2026
- No tags/releases exist (version: 0.0.1)
- 82 open Repo Assist draft PRs (including new PR #130)
- 13 open issues (all have Repo Assist comments)
- No human-authored PRs or recent human activity

## Open Issues
- #15: [agentics] Repo Assist failed — labeled `wontfix`, expired Mar 2026 → suggest close
- #48: [Repo Assist] PR backlog merge order — updated 2026-08-31, covers all 69 PRs (#110-#115 added)
- #49: [Repo Assist] Research: wgpu rendering backend
- #51: [Repo Assist] Research: text metrics debug example
- #94: [Repo Assist] Research: view memoization (implemented as PR #97)
- #98: [Repo Assist] docs(claude.md) issue artifact — suggest close (resolved by PR #99)
- #101–#107: [Repo Assist] ci(bench) push-failed issues — blocked, maintainer must apply manually
- #103: [Repo Assist] Monthly Activity 2026-08 (current month summary)

## Comments Made
All 13 open issues have at least one Repo Assist comment. Do not re-engage unless new human activity appears.
- #48 last updated: 2026-08-31 (added PRs #110-#115 to merge order guide)

## Monthly Summary Issue
- Current: September 2026 (created run 33809160476, issue #103 August closed)
- Previous: #103 (August 2026) — closed 2026-09-03

## Backlog Cursor
- Issue backlog cursor: all issues covered; reset when new issues appear
- PR cursor: all 69 PRs from Repo Assist, no human PRs to nudge

## Round-Robin Task History
- 2026-09-15: Tasks 1, 10, 11
- 2026-09-13: Tasks 4, 5, 11
- 2026-09-06: Tasks 10, 11
- 2026-09-03: Tasks 3, 11
- 2026-08-31: Tasks 5, 10, 11
- 2026-08-30: Tasks 1, 3, 7, 11
- 2026-08-29 run2: Tasks 3, 11
- 2026-08-29 run1: Tasks 10, 11
- 2026-08-28: Tasks 3, 11
- 2026-08-26: Tasks 10, 11
- 2026-08-25: Tasks 1, 5, 7, 11
- 2026-08-24: Tasks 5, 10, 11
- 2026-08-23: Tasks 7, 10, 11
- 2026-08-22: Tasks 1, 2, 6, 9, 11
- 2026-08-21: Tasks 3, 11
- 2026-08-20: Tasks 1, 5, 11
- 2026-08-19: Tasks 1, 5, 8, 11
- 2026-08-18: Tasks 1, 7, 11
- 2026-08-17: Tasks 1, 4, 5, 11

## Priority PRs (conflict-free, ready to merge)
- #36: Multi-click detection (p1 blocker)
- #58: Per-item inline editing TodoMVC (p1)
- #71: Element tree query devtools (p1)
- #118: Edges::axial() / Corners::left_right() symmetric constructors — 2026-09-06
- #116: Edges::new() / Corners::new() explicit constructors — 2026-09-03
- #115: Color helpers (rgb, rgba, gray, rgb_u8, rgba_u8, with_alpha) — 2026-08-31
- #114: DrawContext::paint_h_line + paint_v_line (2026-08-30)
- #113: Quad builder API + DrawContext convenience paint methods (2026-08-29)
- #112: Div layout API — padding_x/y, margin, min/max size (2026-08-29)
- #111: layout_bounds O(depth) iterative fix (perf improvement — 2026-08-28)
- #110: Nested clip intersection fix (correctness bug)
- #39: max_width text wrapping fix (TODO fix)
- #53: Clipboard TODOs fix (TODO fix)
- #92: Dependency updates (refreshed 2026-08-02 with 162 updates)
- #41: opt-level=3 for deps

## PRs to Close
- #20: Superseded by maintainer PR #35
- #21: Superseded by #71

## PR Notes
- GitHub blocks CI triggers for github-actions[bot] PRs via GITHUB_TOKEN — maintainer must manually mark PRs as "Ready for review" to run CI
- All 69 PRs from Repo Assist; no human PRs exist
- Full merge order guide in issue #48 (last updated 2026-08-31, covers all 69 PRs)

## Geometry Constructors Added (2026-09-03)
- **PR #116**: Added explicit per-element constructors to `Edges<T>` and `Corners<T>`:
  - `Edges::new(top, right, bottom, left)`: CSS-order per-side values, no `Copy` bound
  - `Corners::new(tl, tr, br, bl)`: clockwise from top-left per-corner values, no `Copy` bound
- 4 new unit tests; pairs with PR #113 (Quad builder API)
- Infra build failure (Linux missing fontconfig — expected)

## Color Helpers Added (2026-08-31)
- **PR #115**: Added `color` module to `motif_core` with 6 color construction helpers:
  - `rgb(r, g, b)`: opaque color from [0.0, 1.0] floats
  - `rgba(r, g, b, a)`: color with alpha from [0.0, 1.0] floats
  - `gray(v)`: opaque gray from single lightness value
  - `rgb_u8(r, g, b)`: opaque color from [0, 255] u8 values
  - `rgba_u8(r, g, b, a)`: color with alpha from [0, 255] u8 values
  - `with_alpha(color, alpha)`: clone a color with new alpha
- All re-exported from crate root via `pub use color::*`
- 11 unit tests; infra build failure (Linux missing fontconfig - parley dep)

## DrawContext Line Helpers Added (2026-08-30)
- **PR #114**: Added axis-aligned line convenience methods to DrawContext:
  - `paint_h_line(x, y, length, color, thickness)`: horizontal separator/divider
  - `paint_v_line(x, y, length, color, thickness)`: vertical separator/divider
- Both delegate to `paint_quad`; offset, clip, and scale factor apply automatically
- 6 new tests: dimensions at 1× scale, offset accumulation, 2× HiDPI scaling (each method)

## Quad Builder API Added (2026-08-29)
- **PR #113**: Added Quad builder methods and DrawContext convenience paint methods:
  - `Quad::with_corner_radius(f32)`: uniform corner radius
  - `Quad::with_corner_radii(Corners<f32>)`: per-corner radii
  - `Quad::with_border(color, f32)`: uniform border
  - `Quad::with_border_widths(color, Edges<f32>)`: per-edge border
  - `DrawContext::paint_rounded_quad(bounds, fill, radius)`: common rounded rect case
  - `DrawContext::paint_outlined_quad(bounds, fill, border_color, width)`: bordered rect
- 8 new tests (5 in scene.rs, 3 in context.rs)

## Div Layout API Added (2026-08-29)
- **PR #112**: Added 18 new layout methods to `Div`:
  - Asymmetric padding: `padding_x`, `padding_y`, `padding_top/right/bottom/left`
  - Margins: `margin`, `margin_x`, `margin_y`, `margin_top/right/bottom/left`
  - Size constraints: `min_width`, `min_height`, `max_width`, `max_height`
- Fills gap between `padding(f32)` (uniform only) and real-world CSS-like layout needs

## Perf Bug Found and Fixed (2026-08-28)
- **layout_bounds() O(depth²) recursion**: Fixed in PR #111. Iterative O(depth) loop.

## Bug Found and Fixed (2026-08-26)
- **DrawContext::with_clip nested clip bug**: Fixed in PR #110. Inner clips now intersect with outer clips. Three new tests.

## Dependency Audit
- Last audit: 2026-08-02 (PR #92 updated with 162 package updates)
- Next audit: 2026-09-07

## TODO Coverage
- All p1/p2 TODO items: open PRs exist
- P3 "Visual regression test harness": PR #109 (done 2026-08-23)
- P3 "Move hardcoded colors to theme": blocked on PR #44 (Theme trait) merging first
- Inline TODOs: max_width in layout.rs → covered by PR #39; clipboard in playground.rs/todomvc.rs → covered by PR #53

## Bench CI Fix
- Status: BLOCKED — modifying .github/workflows/ requires workflow permission scope
- Maintainer must apply patch from issue #107 or run 31635670794 artifacts
- Then close issues #101, #102, #104, #105, #106, #107

## Future Improvement Ideas
- `PointExt`/`SizeExt` extension traits — `snap_to_pixel()`, `area()`, `max_dimension()` for glamour type aliases
- `Corners::diagonal(tl_br, tr_bl)` done in PR #127
- `Edges::new()` / `Corners::new()` done in PR #116
- `Edges::axial()` / `Corners::left_right()` done in PR #118
- `Edges/Corners::Add/Sub/Mul` done in PR #130

## Edges/Corners Arithmetic Added (2026-09-15)
- **PR #130**: Added `Add`, `Sub`, `Mul<S>` trait impls to `Edges<T>` and `Corners<T>`:
  - `Add`/`Sub` require only `T: Add/Sub<Output=T>` (no Copy bound — consume both operands)
  - `Mul<S>` requires `T: Mul<S,Output=T>` and `S: Copy` — scalar multiplication
  - Useful for combining padding+inset, scaling radii for hover states, etc.
  - 6 new unit tests; fmt passes; Linux fontconfig infra failure expected

## Geometry Axial Constructors Added (2026-09-06)
- **PR #118**: Added `Edges::axial(horizontal, vertical)` and `Corners::left_right(left, right)`:
  - `Edges::axial(h, v)`: x/y axis argument order (horizontal first), complements `symmetric(v, h)` CSS order
  - `Corners::left_right(l, r)`: column-wise symmetry (TL+BL vs TR+BR), complements `top_bottom(t, b)` row-wise
- 4 new unit tests; fmt passes; full macOS build expected to pass on CI

## Structural Notes
- Project: immediate-mode Rust UI framework, Metal GPU backend, macOS only currently
- Codebase: motif, motif_core, motif_debug, motif_debug_cli, motif_test crates
- Uses Taffy for layout, swash for text rasterization, parley for text layout
- Active streams: text-input-rewrite, todomvc, layout, theming, cross-platform, devtools, animation, accessibility
- All major feature areas have open Repo Assist PRs
- Linux build blocked by missing fontconfig (parley dep) — all PRs use infrastructure note
