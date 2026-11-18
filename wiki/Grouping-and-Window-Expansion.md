# Grouping & Window Expansion

Multi-window workflows (e.g. running multiple browser windows, terminal sessions, or document editors) are organized cleanly using **Grouped Application Fan-Out**.

---

## 1. Cascaded Thumbnails

When `grouping_mode = ByApplication`:
- All windows belonging to the same executable are clustered into one stage.
- Up to `max_previews_per_stage` (default 5) preview cards are stacked with slight vertical and horizontal offsets, indicating depth.
- A circular badge on the top corner displays the total count of windows in that group.

---

## 2. Horizontal Fan-Out on Hover

When your cursor enters the bounds of a multi-window stage card:
1. An internal spring parameter `expand_t` animates from `0.0` to `1.0`.
2. The stacked cascaded cards fan out horizontally to the right (or left if docked on the right side).
3. The stacked icon cluster cross-fades into individual Direct2D squircle icons for each window.
4. Each preview card becomes an independent click target. Clicking any specific window raises that window to the foreground immediately.

---

## 3. Hit-Testing Geometry

The hit-test algorithm (`hit_test_stage_or_window` in `src/layout.rs`) dynamically evaluates the cursor coordinates:
- If `expand_t > 0.3`, it tests against the individual expanded card bounding boxes.
- If unexpanded, clicking activates the most recently focused window in the group.
