//! Animación de la isla: expandir al pasar el mouse y contraer al sacarlo.
//!
//! Es lógica pura, sin código de ningún sistema operativo: el backend le avisa
//! cuándo entra o sale el mouse y le pide el tamaño en cada tick del timer.
//! El timer solo existe mientras dura la animación, así que en reposo no hay
//! ningún trabajo pendiente.

use std::time::{Duration, Instant};

use atoll_render::Size;

/// Curva de aceleración: arranca rápido y frena suave (`ease-out cubic`).
///
/// Recibe el progreso `t` y devuelve el progreso suavizado, ambos en `0.0..=1.0`.
#[must_use]
pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// Resultado de avanzar la animación un tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Step {
    /// Tamaño que debe tener la isla en este instante.
    pub size: Size,
    /// `true` cuando la animación terminó y el timer se puede apagar.
    pub finished: bool,
}

/// Máquina de estados de la animación de hover.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Animator {
    collapsed: Size,
    expanded: Size,
    duration: Duration,
    hovering: bool,
    animating: bool,
    start: Instant,
    from: (f32, f32),
    to: (f32, f32),
    current: Size,
}

impl Animator {
    /// Crea un animador en reposo, con la isla contraída.
    #[must_use]
    pub fn new(collapsed: Size, expanded: Size, duration: Duration, now: Instant) -> Self {
        Self {
            collapsed,
            expanded,
            duration,
            hovering: false,
            animating: false,
            start: now,
            from: as_floats(collapsed),
            to: as_floats(collapsed),
            current: collapsed,
        }
    }

    /// Tamaño actual de la isla.
    #[must_use]
    pub const fn size(&self) -> Size {
        self.current
    }

    /// Indica si el mouse está sobre la isla.
    #[must_use]
    pub const fn is_hovering(&self) -> bool {
        self.hovering
    }

    /// Informa que el mouse entró (`true`) o salió (`false`) de la isla.
    ///
    /// Devuelve `true` si hay que arrancar el timer, es decir, si el estado
    /// cambió. Si ya había una animación en curso, la nueva parte desde el
    /// tamaño actual, sin saltos.
    pub fn set_hover(&mut self, hovering: bool, now: Instant) -> bool {
        if self.hovering == hovering {
            return false;
        }
        self.hovering = hovering;
        self.from = as_floats(self.current);
        self.to = as_floats(if hovering {
            self.expanded
        } else {
            self.collapsed
        });
        self.start = now;
        self.animating = true;
        true
    }

    /// Reconfigura los tamaños de reposo y expandido (por ejemplo al recargar
    /// la configuración o cambiar el DPI).
    ///
    /// No anima: cancela cualquier animación en curso y salta de inmediato al
    /// tamaño nuevo que corresponde al estado actual (expandido si el mouse está
    /// encima, contraído si no). El estado de hover se conserva.
    pub fn reconfigure(&mut self, collapsed: Size, expanded: Size) {
        self.collapsed = collapsed;
        self.expanded = expanded;
        self.current = if self.hovering { expanded } else { collapsed };
        self.animating = false;
        self.from = as_floats(self.current);
        self.to = self.from;
    }

    /// Cambia los tamaños de reposo y expandido, por ejemplo cuando cambia el DPI.
    ///
    /// Equivale a [`Self::reconfigure`]: no anima y conserva el estado de hover.
    pub fn resize(&mut self, collapsed: Size, expanded: Size) {
        self.reconfigure(collapsed, expanded);
    }

    /// Avanza la animación hasta `now` y devuelve el tamaño correspondiente.
    pub fn tick(&mut self, now: Instant) -> Step {
        if !self.animating {
            return Step {
                size: self.current,
                finished: true,
            };
        }

        let elapsed = now.saturating_duration_since(self.start);
        let progress = if self.duration.is_zero() {
            1.0
        } else {
            (elapsed.as_secs_f32() / self.duration.as_secs_f32()).min(1.0)
        };
        let eased = ease_out_cubic(progress);

        self.current = Size::new(
            lerp(self.from.0, self.to.0, eased),
            lerp(self.from.1, self.to.1, eased),
        );
        if progress >= 1.0 {
            self.animating = false;
        }
        Step {
            size: self.current,
            finished: !self.animating,
        }
    }
}

/// Convierte un tamaño a pares de `f32` para interpolar.
fn as_floats(size: Size) -> (f32, f32) {
    (size.width as f32, size.height as f32)
}

/// Interpola linealmente entre `a` y `b` y redondea a píxeles enteros.
fn lerp(a: f32, b: f32, t: f32) -> u32 {
    (a + (b - a) * t).round().max(0.0) as u32
}

#[cfg(test)]
mod tests {
    use super::{ease_out_cubic, Animator};
    use atoll_render::Size;
    use std::time::{Duration, Instant};

    const SMALL: Size = Size::new(400, 40);
    const BIG: Size = Size::new(480, 64);
    const DURATION: Duration = Duration::from_millis(200);

    fn animator(now: Instant) -> Animator {
        Animator::new(SMALL, BIG, DURATION, now)
    }

    #[test]
    fn easing_has_fixed_endpoints_and_never_decreases() {
        assert!(ease_out_cubic(0.0).abs() < f32::EPSILON);
        assert!((ease_out_cubic(1.0) - 1.0).abs() < f32::EPSILON);
        let mut previous = 0.0;
        for step in 0..=100 {
            let value = ease_out_cubic(step as f32 / 100.0);
            assert!(value >= previous);
            previous = value;
        }
    }

    #[test]
    fn starts_collapsed_and_idle() {
        let t0 = Instant::now();
        let mut anim = animator(t0);
        assert_eq!(anim.size(), SMALL);
        assert!(!anim.is_hovering());
        assert!(anim.tick(t0).finished);
    }

    #[test]
    fn hover_expands_until_finished() {
        let t0 = Instant::now();
        let mut anim = animator(t0);
        assert!(anim.set_hover(true, t0));

        let middle = anim.tick(t0 + Duration::from_millis(100));
        assert!(!middle.finished);
        assert!(middle.size.width > SMALL.width && middle.size.width < BIG.width);
        assert!(middle.size.height > SMALL.height && middle.size.height < BIG.height);

        let end = anim.tick(t0 + Duration::from_millis(250));
        assert!(end.finished);
        assert_eq!(end.size, BIG);
    }

    #[test]
    fn repeated_hover_does_not_restart_the_timer() {
        let t0 = Instant::now();
        let mut anim = animator(t0);
        assert!(anim.set_hover(true, t0));
        assert!(!anim.set_hover(true, t0));
    }

    #[test]
    fn leaving_mid_animation_continues_from_current_size() {
        let t0 = Instant::now();
        let mut anim = animator(t0);
        anim.set_hover(true, t0);
        let half = anim.tick(t0 + Duration::from_millis(100)).size;

        let t1 = t0 + Duration::from_millis(100);
        assert!(anim.set_hover(false, t1));
        assert_eq!(anim.tick(t1).size, half);

        let end = anim.tick(t1 + Duration::from_millis(300));
        assert!(end.finished);
        assert_eq!(end.size, SMALL);
    }

    #[test]
    fn zero_duration_finishes_immediately() {
        let t0 = Instant::now();
        let mut anim = Animator::new(SMALL, BIG, Duration::ZERO, t0);
        anim.set_hover(true, t0);
        let step = anim.tick(t0);
        assert!(step.finished);
        assert_eq!(step.size, BIG);
    }

    #[test]
    fn resize_while_collapsed_snaps_to_the_new_collapsed_size() {
        let now = Instant::now();
        let mut animator = Animator::new(Size::new(400, 40), Size::new(480, 64), DURATION, now);
        animator.resize(Size::new(600, 60), Size::new(720, 96));
        assert_eq!(animator.size(), Size::new(600, 60));
        assert!(!animator.is_hovering());
        let step = animator.tick(now);
        assert!(step.finished);
        assert_eq!(step.size, Size::new(600, 60));
    }

    #[test]
    fn resize_while_hovering_keeps_the_hover_and_snaps_to_the_new_expanded_size() {
        let now = Instant::now();
        let mut animator = Animator::new(Size::new(400, 40), Size::new(480, 64), DURATION, now);
        animator.set_hover(true, now);
        animator.tick(now + DURATION / 2);

        animator.resize(Size::new(600, 60), Size::new(720, 96));
        assert!(animator.is_hovering());
        assert_eq!(animator.size(), Size::new(720, 96));
        assert!(
            animator.tick(now + DURATION).finished,
            "no queda animación pendiente"
        );
        assert_eq!(animator.size(), Size::new(720, 96));
    }

    #[test]
    fn after_a_resize_the_next_hover_change_animates_between_the_new_sizes() {
        let now = Instant::now();
        let mut animator = Animator::new(Size::new(400, 40), Size::new(480, 64), DURATION, now);
        animator.resize(Size::new(600, 60), Size::new(720, 96));

        assert!(animator.set_hover(true, now));
        let end = animator.tick(now + DURATION);
        assert!(end.finished);
        assert_eq!(end.size, Size::new(720, 96));

        assert!(animator.set_hover(false, now + DURATION));
        let back = animator.tick(now + DURATION * 2);
        assert_eq!(back.size, Size::new(600, 60));
    }

    #[test]
    fn reconfigure_snaps_immediately_and_preserves_hover_state() {
        let now = Instant::now();
        let mut anim = Animator::new(SMALL, BIG, DURATION, now);

        // En reposo: salta al nuevo tamaño contraído sin animar.
        anim.reconfigure(Size::new(360, 36), Size::new(520, 72));
        assert!(!anim.is_hovering());
        assert_eq!(anim.size(), Size::new(360, 36));
        assert!(anim.tick(now).finished);

        // Con hover a mitad de camino: salta al nuevo tamaño expandido y apaga la animación.
        assert!(anim.set_hover(true, now));
        anim.tick(now + DURATION / 3);
        anim.reconfigure(Size::new(420, 44), Size::new(600, 80));
        assert!(anim.is_hovering());
        assert_eq!(anim.size(), Size::new(600, 80));
        assert!(anim.tick(now + DURATION).finished);
    }
}
