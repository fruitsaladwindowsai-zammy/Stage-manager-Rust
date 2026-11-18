# Stage Mechanics & Group Fan-Out

Stage Manager organizes your applications into fluid stages with automatic grouping and horizontal fan-out.

---

## 1. What is a Stage?

- A **Stage** represents an active workspace containing one or more related application windows.
- Inactive stages float in an auto-hiding ribbon on the side of your display, showing live hardware thumbnails.
- Clicking an inactive stage seamlessly swaps it into focus while retracting the previously active stage back onto the strip.

---

## 2. Multi-Window Grouping & Fan-Out

When `grouping_mode = ByApplication`:
- All windows belonging to the same executable (e.g. multiple Chrome tabs or VS Code editors) are clustered into one stage.
- Up to 5 cascaded preview thumbnails are stacked with depth offsets.
- **Horizontal Fan-Out:** Hovering over the card fans out the windows horizontally with individual squircle icons. Clicking any specific window brings that window directly to the front.

---

## 3. Dynamic Merging & Splitting

- **Merge:** Hold `Shift` and left-click an inactive stage card. It will immediately join your current active stage rather than replacing it.
- **Separate:** Drag any window titlebar away from the grouped layout, and Stage Manager will split it back into an independent stage.
