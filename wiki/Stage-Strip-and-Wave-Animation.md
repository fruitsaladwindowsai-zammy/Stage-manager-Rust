# Stage Strip & Wave Animation

Stage Manager features a signature macOS-style curved ribbon wave and infinite vertical carousel.

---

## 1. Damped Spring Physics

Transitions do not use fixed linear or cubic bezier easing curves. Instead, every element is attached to a simulated damped spring:

$$\frac{d^2 x}{dt^2} + 2\zeta\omega_n \frac{dx}{dt} + \omega_n^2 (x - x_{\text{target}}) = 0$$

- **Stiffness ($k = 240.0$):** Determines how rapidly the card accelerates toward the target coordinate.
- **Damping ($c = 26.0$):** Controls how gently the movement settles, preventing unnatural oscillations or hard stops.

If the user quickly changes cursor direction mid-flight, the spring dynamically redirects without stuttering or snapping.

---

## 2. Inward Curved Ribbon Wave

When you hover over a stage card along the vertical strip:
1. The **hovered card** pops forward toward the user (`+12px` scale and subtle illumination glow).
2. The **neighboring cards** immediately above and below curve inward toward the screen edge:
   $$\Delta x_i = -A \cdot \exp\left(-\frac{(y_i - y_{\text{cursor}})^2}{2\sigma^2}\right)$$
3. As the cursor glides up or down, the wave rolls along the stage ribbon smoothly.

---

## 3. Infinite Carousel Loop

When the number of open stages exceeds the vertical screen height:
- Stages outside the visible bounds wrap seamlessly around to the opposing edge.
- Hovering near the top or bottom 40px of the screen triggers continuous, smooth carousel scrolling.
- Hit-testing accounts for wrapped positions so clicking any partially visible card activates the correct stage.
