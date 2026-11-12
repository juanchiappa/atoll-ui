//! Animación de la isla y lógica del retardo de hover.
//!
//! Es lógica pura, sin código de ningún sistema operativo: el backend le avisa
//! cuándo entra o sale el mouse, cuándo vence el retardo de hover y le pide el
//! tamaño en cada tick del timer de animación. Ningún timer queda activo en reposo.

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

/// Acción que el backend debe ejecutar tras un evento del mouse o del timer de retardo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoverAction {
    /// No hay nada que hacer (por ejemplo, movimiento del mouse cuando ya estaba adentro).
    None,
    /// Armar el timer de un solo disparo del retardo de hover por `delay_ms` milisegundos.
    ArmDelayTimer {
        /// Milisegundos a esperar con el mouse adentro antes de empezar a expandir.
        delay_ms: u32,
    },
    /// Cancelar el timer de retardo porque el mouse salió antes de que venciera.
    CancelDelayTimer,
    /// Empezar la animación de expansión de la isla.
    StartExpand,
    /// Empezar la animación de contracción de la isla (siempre sin retardo).
    StartCollapse,
}

/// Máquina de estados de la animación y del retardo de hover.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Animator {
    collapsed: Size,
    expanded: Size,
    duration: Duration,
    hover_delay_ms: u32,
    pointer_inside: bool,
    delay_pending: bool,
    hovering: bool,
    animating: bool,
    start: Instant,
    from: (f32, f32),
    to: (f32, f32),
    current: Size,
}

impl Animator {
    /// Crea un animador en reposo, con la isla contraída y sin retardo de hover.
    #[must_use]
    pub fn new(collapsed: Size, expanded: Size, duration: Duration, now: Instant) -> Self {
        Self {
            collapsed,
            expanded,
            duration,
            hover_delay_ms: 0,
            pointer_inside: false,
            delay_pending: false,
            hovering: false,
            animating: false,
            start: now,
            from: as_floats(collapsed),
            to: as_floats(collapsed),
            current: collapsed,
        }
    }

    /// Configura el retardo de hover (`hover_delay_ms`) y la duración de la animación (`duration`).
    #[must_use]
    pub const fn with_timing(mut self, hover_delay_ms: u32, duration: Duration) -> Self {
        self.hover_delay_ms = hover_delay_ms;
        self.duration = duration;
        self
    }

    /// Actualiza en caliente el retardo de hover y la duración de la animación.
    pub fn set_timing(&mut self, hover_delay_ms: u32, duration: Duration) {
        self.hover_delay_ms = hover_delay_ms;
        self.duration = duration;
    }

    /// Duración configurada para expandir y contraer.
    #[must_use]
    pub const fn duration(&self) -> Duration {
        self.duration
    }

    /// Retardo configurado antes de expandir al entrar el mouse, en milisegundos.
    #[must_use]
    pub const fn hover_delay_ms(&self) -> u32 {
        self.hover_delay_ms
    }

    /// Tamaño actual de la isla.
    #[must_use]
    pub const fn size(&self) -> Size {
        self.current
    }

    /// Indica si la isla está en estado expandido (o expandiéndose).
    #[must_use]
    pub const fn is_hovering(&self) -> bool {
        self.hovering
    }

    /// Indica si el puntero del mouse está actualmente dentro de la isla.
    #[must_use]
    pub const fn is_pointer_inside(&self) -> bool {
        self.pointer_inside
    }

    /// Registra que el mouse entró (o se movió dentro) de la isla y devuelve la acción a tomar.
    ///
    /// - Si el mouse ya estaba adentro, devuelve [`HoverAction::None`].
    /// - Si `hover_delay_ms == 0`, arranca la expansión de inmediato ([`HoverAction::StartExpand`]).
    /// - Si `hover_delay_ms > 0`, pide armar el timer de un solo disparo ([`HoverAction::ArmDelayTimer`]).
    pub fn on_mouse_enter(&mut self, now: Instant) -> HoverAction {
        if self.pointer_inside {
            return HoverAction::None;
        }
        self.pointer_inside = true;

        if self.hovering {
            self.delay_pending = false;
            return HoverAction::None;
        }

        if self.hover_delay_ms == 0 {
            self.delay_pending = false;
            self.start_transition(true, now);
            HoverAction::StartExpand
        } else {
            self.delay_pending = true;
            HoverAction::ArmDelayTimer {
                delay_ms: self.hover_delay_ms,
            }
        }
    }

    /// Registra que el mouse salió de la isla y devuelve la acción a tomar.
    ///
    /// - Si había un retardo pendiente sin vencer, lo cancela ([`HoverAction::CancelDelayTimer`])
    ///   y la isla no llega a expandirse.
    /// - Si la isla estaba expandida o expandiéndose, empieza a contraer de inmediato sin retardo
    ///   ([`HoverAction::StartCollapse`]).
    pub fn on_mouse_leave(&mut self, now: Instant) -> HoverAction {
        self.pointer_inside = false;

        if self.delay_pending {
            self.delay_pending = false;
            return HoverAction::CancelDelayTimer;
        }

        if self.hovering {
            self.start_transition(false, now);
            return HoverAction::StartCollapse;
        }

        HoverAction::None
    }

    /// Notifica que venció el timer del retardo de hover.
    ///
    /// Si el mouse sigue adentro y el retardo seguía pendiente, comienza a expandir la isla
    /// ([`HoverAction::StartExpand`]); en caso contrario devuelve [`HoverAction::None`].
    pub fn on_delay_elapsed(&mut self, now: Instant) -> HoverAction {
        if !self.delay_pending || !self.pointer_inside {
            self.delay_pending = false;
            return HoverAction::None;
        }
        self.delay_pending = false;
        if self.hovering {
            return HoverAction::None;
        }
        self.start_transition(true, now);
        HoverAction::StartExpand
    }

    /// Informa directamente que el estado de hover cambió a `hovering` (sin retardo).
    ///
    /// Devuelve `true` si hay que arrancar la animación, es decir, si el estado
    /// cambió. Si ya había una animación en curso, la nueva parte desde el
    /// tamaño actual, sin saltos.
    pub fn set_hover(&mut self, hovering: bool, now: Instant) -> bool {
        self.pointer_inside = hovering;
        self.delay_pending = false;
        if self.hovering == hovering {
            return false;
        }
        self.start_transition(hovering, now);
        true
    }

    /// Inicia una transición hacia expandido (`true`) o contraído (`false`) desde el tamaño actual.
    fn start_transition(&mut self, hovering: bool, now: Instant) {
        self.hovering = hovering;
        self.from = as_floats(self.current);
        self.to = as_floats(if hovering {
            self.expanded
        } else {
            self.collapsed
        });
        self.start = now;
        self.animating = true;
    }

    /// Reconfigura los tamaños de reposo y expandido (por ejemplo al recargar
    /// la configuración o cambiar el DPI).
    ///
    /// No anima: cancela cualquier animación o retardo en curso y salta de inmediato al
    /// tamaño nuevo que corresponde al estado actual (expandido si el hover ya estaba
    /// activo, contraído si no). El estado de hover se conserva.
    pub fn reconfigure(&mut self, collapsed: Size, expanded: Size) {
        self.collapsed = collapsed;
        self.expanded = expanded;
        self.current = if self.hovering { expanded } else { collapsed };
        self.animating = false;
        self.delay_pending = false;
        self.pointer_inside = self.hovering;
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
    use super::{ease_out_cubic, Animator, HoverAction};
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
        assert!(!anim.is_pointer_inside());
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

    #[test]
    fn zero_delay_expands_on_enter_and_collapses_on_leave_immediately() {
        let t0 = Instant::now();
        let mut anim = Animator::new(SMALL, BIG, DURATION, t0).with_timing(0, DURATION);

        assert_eq!(anim.on_mouse_enter(t0), HoverAction::StartExpand);
        assert!(anim.is_hovering());
        assert!(anim.is_pointer_inside());
        // Movimientos repetidos dentro de la isla no rearman nada.
        assert_eq!(anim.on_mouse_enter(t0), HoverAction::None);

        let t1 = t0 + Duration::from_millis(100);
        assert_eq!(anim.on_mouse_leave(t1), HoverAction::StartCollapse);
        assert!(!anim.is_hovering());
        assert!(!anim.is_pointer_inside());
    }

    #[test]
    fn hover_delay_arms_timer_and_leaving_before_expiry_cancels_without_expanding() {
        let t0 = Instant::now();
        let mut anim = Animator::new(SMALL, BIG, DURATION, t0).with_timing(400, DURATION);

        assert_eq!(
            anim.on_mouse_enter(t0),
            HoverAction::ArmDelayTimer { delay_ms: 400 }
        );
        assert!(!anim.is_hovering());
        assert!(anim.is_pointer_inside());
        assert_eq!(anim.size(), SMALL);

        // Movimientos repetidos mientras corre el retardo no rearman el timer.
        assert_eq!(
            anim.on_mouse_enter(t0 + Duration::from_millis(50)),
            HoverAction::None
        );

        // El mouse sale antes de los 400 ms: se cancela el timer y nunca llega a expandir.
        let t_leave = t0 + Duration::from_millis(150);
        assert_eq!(anim.on_mouse_leave(t_leave), HoverAction::CancelDelayTimer);
        assert!(!anim.is_hovering());
        assert!(!anim.is_pointer_inside());
        assert_eq!(anim.size(), SMALL);
        assert!(anim.tick(t_leave).finished);

        // Si por carrera llegara un disparo tardío del timer, se ignora.
        assert_eq!(
            anim.on_delay_elapsed(t0 + Duration::from_millis(400)),
            HoverAction::None
        );
        assert!(!anim.is_hovering());
    }

    #[test]
    fn hover_delay_expands_when_elapsed_and_collapses_without_delay_on_leave() {
        let t0 = Instant::now();
        let mut anim = Animator::new(SMALL, BIG, DURATION, t0).with_timing(400, DURATION);

        assert_eq!(
            anim.on_mouse_enter(t0),
            HoverAction::ArmDelayTimer { delay_ms: 400 }
        );

        let t_elapsed = t0 + Duration::from_millis(400);
        assert_eq!(anim.on_delay_elapsed(t_elapsed), HoverAction::StartExpand);
        assert!(anim.is_hovering());

        // Avanza la expansión hasta terminar.
        let end = anim.tick(t_elapsed + DURATION);
        assert!(end.finished);
        assert_eq!(end.size, BIG);

        // Al salir el mouse, contrae de inmediato sin ningún retardo.
        let t_leave = t_elapsed + DURATION + Duration::from_millis(50);
        assert_eq!(anim.on_mouse_leave(t_leave), HoverAction::StartCollapse);
        assert!(!anim.is_hovering());
        let collapsed = anim.tick(t_leave + DURATION);
        assert!(collapsed.finished);
        assert_eq!(collapsed.size, SMALL);
    }
}
