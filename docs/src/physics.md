# Physics & 120Hz Dynamics

Every visual movement in Stage Manager is calculated using second-order damped spring equations synchronized with the hardware vertical blank.

---

## 1. Damped Spring Equations

Unlike fixed linear or bezier easing curves, damped springs adapt naturally to user interaction:

$$\frac{d^2 x}{dt^2} + 2\zeta\omega_n \frac{dx}{dt} + \omega_n^2 (x - x_{\text{target}}) = 0$$

- **Stiffness ($k = 240.0$):** Governs how quickly cards accelerate toward target coordinates.
- **Damping ($c = 26.0$):** Controls how gently movement settles, preventing unnatural overshoot.

If you suddenly change cursor direction mid-flight, the spring dynamically redirects without stuttering or snapping.

---

## 2. Inward Curved Ribbon Wave

When you hover over a stage card:
1. The **hovered card** pops forward toward you (`+12px` scale and subtle illumination glow).
2. The **neighboring cards** immediately above and below curve inward toward the screen bezel:
   $$\Delta x_i = -A \cdot \exp\left(-\frac{(y_i - y_{\text{cursor}})^2}{2\sigma^2}\right)$$
3. As the cursor glides vertically, the wave rolls along the stage ribbon smoothly.

---

## 3. VSync Synchronization via `DwmFlush`

The animation loop calls `DwmFlush()` on a background thread:
- `DwmFlush()` blocks until the Windows DWM compositor performs a vertical blank.
- This ensures frame calculations align with 60Hz, 120Hz, or 144Hz display refresh rates.
- When springs settle to target coordinates, the animation thread transitions to an idle state, consuming **0% CPU**.
