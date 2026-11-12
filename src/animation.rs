//! Spring physics and DWM VSync ticker for silky-smooth 60–144Hz+ animations.
//!
//! Instead of fixed-duration timers that jump when interrupted, every visual
//! property (sidebar X slide, card Y offset, hover scale, thumbnail opacity,
//! transition window bounds) is driven by a 2nd-order damped spring stepped
//! in lockstep with `DwmFlush()` (the DWM compositor's VBlank signal).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::DwmFlush;
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_APP};

/// Custom message posted to the main window on each DWM VBlank when animations are active.
pub const WM_STAGE_VSYNC: u32 = WM_APP + 10;

/// A 2nd-order damped spring for smooth, interruptible UI motion.
#[derive(Debug, Clone, Copy)]
pub struct Spring {
    pub value: f64,
    pub target: f64,
    pub velocity: f64,
    pub stiffness: f64,
    pub damping: f64,
}

impl Spring {
    pub fn new(initial: f64, stiffness: f64, damping: f64) -> Self {
        Self {
            value: initial,
            target: initial,
            velocity: 0.0,
            stiffness,
            damping,
        }
    }

    /// Snappy spring suitable for stage transitions and sidebar slides.
    pub fn snappy(initial: f64) -> Self {
        Self::new(initial, 320.0, 32.0)
    }

    /// Extra responsive spring for hover zoom and card highlights.
    pub fn hover(initial: f64) -> Self {
        Self::new(initial, 440.0, 36.0)
    }

    /// Smooth spring for card vertical reflows.
    pub fn reflow(initial: f64) -> Self {
        Self::new(initial, 260.0, 30.0)
    }

    /// Redirect the spring to a new target without resetting current position or velocity.
    pub fn set_target(&mut self, target: f64) {
        self.target = target;
    }

    /// Advance the spring by `dt` seconds using fixed sub-steps for stability.
    /// Returns `true` if still moving, `false` if settled.
    pub fn step(&mut self, dt: f64) -> bool {
        if self.is_settled() {
            self.value = self.target;
            self.velocity = 0.0;
            return false;
        }

        // Clamp dt (e.g. after breakpoint or system wake) and sub-step at max 4ms
        let clamped_dt = dt.clamp(0.0001, 0.05);
        let max_sub = 0.004;
        let steps = ((clamped_dt / max_sub).ceil() as usize).max(1);
        let sub_dt = clamped_dt / steps as f64;

        for _ in 0..steps {
            let force = (self.target - self.value) * self.stiffness;
            let drag = self.velocity * self.damping;
            let accel = force - drag;
            self.velocity += accel * sub_dt;
            self.value += self.velocity * sub_dt;
        }

        if self.is_settled() {
            self.value = self.target;
            self.velocity = 0.0;
            false
        } else {
            true
        }
    }

    /// True if position and velocity are within settling epsilon.
    pub fn is_settled(&self) -> bool {
        (self.target - self.value).abs() < 0.15 && self.velocity.abs() < 0.8
    }

    /// Settling check for normalized 0.0..1.0 properties (opacity, scale).
    pub fn step_normalized(&mut self, dt: f64) -> bool {
        if (self.target - self.value).abs() < 0.002 && self.velocity.abs() < 0.01 {
            self.value = self.target;
            self.velocity = 0.0;
            return false;
        }

        let clamped_dt = dt.clamp(0.0001, 0.05);
        let max_sub = 0.004;
        let steps = ((clamped_dt / max_sub).ceil() as usize).max(1);
        let sub_dt = clamped_dt / steps as f64;

        for _ in 0..steps {
            let force = (self.target - self.value) * self.stiffness;
            let drag = self.velocity * self.damping;
            let accel = force - drag;
            self.velocity += accel * sub_dt;
            self.value += self.velocity * sub_dt;
        }

        if (self.target - self.value).abs() < 0.002 && self.velocity.abs() < 0.01 {
            self.value = self.target;
            self.velocity = 0.0;
            false
        } else {
            true
        }
    }
}

/// Four-component spring for smoothly animating a `RECT`.
#[derive(Debug, Clone, Copy)]
pub struct SpringRect {
    pub left: Spring,
    pub top: Spring,
    pub right: Spring,
    pub bottom: Spring,
}

impl SpringRect {
    pub fn new(initial: RECT, stiffness: f64, damping: f64) -> Self {
        Self {
            left: Spring::new(initial.left as f64, stiffness, damping),
            top: Spring::new(initial.top as f64, stiffness, damping),
            right: Spring::new(initial.right as f64, stiffness, damping),
            bottom: Spring::new(initial.bottom as f64, stiffness, damping),
        }
    }

    pub fn set_target(&mut self, target: RECT) {
        self.left.set_target(target.left as f64);
        self.top.set_target(target.top as f64);
        self.right.set_target(target.right as f64);
        self.bottom.set_target(target.bottom as f64);
    }

    pub fn step(&mut self, dt: f64) -> bool {
        let a = self.left.step(dt);
        let b = self.top.step(dt);
        let c = self.right.step(dt);
        let d = self.bottom.step(dt);
        a || b || c || d
    }

    pub fn current_rect(&self) -> RECT {
        RECT {
            left: self.left.value.round() as i32,
            top: self.top.value.round() as i32,
            right: self.right.value.round() as i32,
            bottom: self.bottom.value.round() as i32,
        }
    }
}

/// Drives frame-accurate animation ticks synced to the Windows DWM VBlank.
///
/// Consumes 0% CPU while idle, waking immediately when `wake()` is called.
pub struct VsyncTicker {
    state: Arc<(Mutex<bool>, Condvar)>,
    running: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl VsyncTicker {
    /// Spawn the background VSync ticker targeting `hwnd`.
    pub fn new(hwnd: HWND) -> Self {
        let state = Arc::new((Mutex::new(false), Condvar::new()));
        let running = Arc::new(AtomicBool::new(true));

        let hwnd_raw = hwnd.0 as isize;
        let state_bg = Arc::clone(&state);
        let running_bg = Arc::clone(&running);

        let handle = thread::Builder::new()
            .name("stage-vsync".to_string())
            .spawn(move || {
                let target = HWND(hwnd_raw as *mut _);
                let (lock, cvar) = &*state_bg;

                while running_bg.load(Ordering::Acquire) {
                    {
                        let mut active = lock.lock().unwrap();
                        while !*active && running_bg.load(Ordering::Acquire) {
                            active = cvar.wait(active).unwrap();
                        }
                    }

                    if !running_bg.load(Ordering::Acquire) {
                        break;
                    }

                    // Wait for next DWM compositor vertical blank
                    let flush_res = unsafe { DwmFlush() };
                    if flush_res.is_err() {
                        thread::sleep(Duration::from_millis(8));
                    }

                    if !running_bg.load(Ordering::Acquire) {
                        break;
                    }

                    // Only post if still active
                    let active = *lock.lock().unwrap();
                    if active {
                        unsafe {
                            let _ = PostMessageW(target, WM_STAGE_VSYNC, WPARAM(0), LPARAM(0));
                        }
                    }
                }
            })
            .expect("Failed to spawn VSync thread");

        Self {
            state,
            running,
            handle: Some(handle),
        }
    }

    /// Wake the VSync loop (call whenever any animation starts).
    pub fn set_active(&self, active: bool) {
        let (lock, cvar) = &*self.state;
        let mut guard = lock.lock().unwrap();
        if *guard != active {
            *guard = active;
            if active {
                cvar.notify_one();
            }
        }
    }
}

impl Drop for VsyncTicker {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Release);
        {
            let (lock, cvar) = &*self.state;
            let mut guard = lock.lock().unwrap();
            *guard = false;
            cvar.notify_all();
        }
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

/// Tracks frame delta-time across vsync ticks.
pub struct FrameClock {
    last: Option<Instant>,
}

impl FrameClock {
    pub fn new() -> Self {
        Self { last: None }
    }

    /// Measure elapsed seconds since the previous active frame, resetting cleanly on wake.
    pub fn tick(&mut self) -> f64 {
        let now = Instant::now();
        let dt = match self.last {
            Some(prev) => now.duration_since(prev).as_secs_f64(),
            None => 1.0 / 60.0,
        };
        self.last = Some(now);
        dt
    }

    pub fn reset(&mut self) {
        self.last = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spring_converges_to_target() {
        let mut s = Spring::snappy(0.0);
        s.set_target(200.0);
        for _ in 0..120 {
            s.step(1.0 / 60.0);
        }
        assert!((s.value - 200.0).abs() < 0.2);
        assert!(s.is_settled());
    }

    #[test]
    fn spring_redirects_smoothly_mid_flight() {
        let mut s = Spring::snappy(0.0);
        s.set_target(200.0);
        for _ in 0..10 {
            s.step(1.0 / 60.0);
        }
        let mid = s.value;
        assert!(mid > 10.0 && mid < 200.0);

        s.set_target(50.0);
        for _ in 0..180 {
            s.step(1.0 / 60.0);
        }
        assert!((s.value - 50.0).abs() < 0.2);
    }

    #[test]
    fn spring_rect_converges() {
        let mut sr = SpringRect::new(
            RECT {
                left: 0,
                top: 0,
                right: 100,
                bottom: 100,
            },
            320.0,
            32.0,
        );
        sr.set_target(RECT {
            left: 100,
            top: 200,
            right: 500,
            bottom: 600,
        });
        for _ in 0..180 {
            sr.step(1.0 / 60.0);
        }
        let r = sr.current_rect();
        assert_eq!(r.left, 100);
        assert_eq!(r.top, 200);
        assert_eq!(r.right, 500);
        assert_eq!(r.bottom, 600);
    }
}
