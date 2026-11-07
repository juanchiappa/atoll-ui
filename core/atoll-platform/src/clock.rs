//! Hora local.
//!
//! La hora se pide al sistema operativo solo cuando hace falta: al dibujar y al
//! disparar el timer del reloj (uno por minuto). No hay polling. La lógica de
//! "cuánto falta para el próximo minuto" es pura y vive acá para poder probarla
//! en cualquier sistema operativo.

use crate::PlatformError;

/// Duración de un minuto en milisegundos.
const MINUTE_MS: u32 = 60_000;

/// Fecha y hora local, con precisión de milisegundos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalTime {
    /// Año (por ejemplo, 2026).
    pub year: u16,
    /// Mes, de 1 (enero) a 12 (diciembre).
    pub month: u8,
    /// Día del mes, de 1 a 31.
    pub day: u8,
    /// Día de la semana, de 0 (domingo) a 6 (sábado).
    pub weekday: u8,
    /// Hora, de 0 a 23.
    pub hour: u8,
    /// Minuto, de 0 a 59.
    pub minute: u8,
    /// Segundo, de 0 a 59.
    pub second: u8,
    /// Milisegundo, de 0 a 999.
    pub millisecond: u16,
}

impl LocalTime {
    /// Milisegundos que faltan para el próximo cambio de minuto (de 1 a 60 000).
    #[must_use]
    pub fn millis_until_next_minute(&self) -> u32 {
        let elapsed = u32::from(self.second) * 1000 + u32::from(self.millisecond);
        MINUTE_MS.saturating_sub(elapsed).max(1)
    }

    /// `true` si `other` cae en el mismo minuto del mismo día.
    #[must_use]
    pub fn same_minute(&self, other: &Self) -> bool {
        (self.year, self.month, self.day, self.hour, self.minute)
            == (other.year, other.month, other.day, other.hour, other.minute)
    }
}

/// Devuelve la fecha y hora local del sistema.
///
/// # Errors
///
/// Devuelve [`PlatformError::Unsupported`] si todavía no hay backend para este
/// sistema operativo. En Windows no falla.
#[cfg(windows)]
pub fn local_time() -> Result<LocalTime, PlatformError> {
    use std::mem::zeroed;

    use windows_sys::Win32::Foundation::SYSTEMTIME;
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;

    // SAFETY: `SYSTEMTIME` es un struct de datos planos; cero es un valor válido.
    let mut raw: SYSTEMTIME = unsafe { zeroed() };
    // SAFETY: `raw` es un puntero válido; `GetLocalTime` no tiene otras precondiciones.
    unsafe { GetLocalTime(&mut raw) };

    Ok(LocalTime {
        year: raw.wYear,
        month: u8::try_from(raw.wMonth).unwrap_or(1),
        day: u8::try_from(raw.wDay).unwrap_or(1),
        weekday: u8::try_from(raw.wDayOfWeek).unwrap_or(0),
        hour: u8::try_from(raw.wHour).unwrap_or(0),
        minute: u8::try_from(raw.wMinute).unwrap_or(0),
        second: u8::try_from(raw.wSecond).unwrap_or(0),
        millisecond: raw.wMilliseconds,
    })
}

/// Devuelve la fecha y hora local del sistema.
///
/// # Errors
///
/// Devuelve [`PlatformError::Unsupported`] si todavía no hay backend para este
/// sistema operativo. En Windows no falla.
#[cfg(not(windows))]
pub fn local_time() -> Result<LocalTime, PlatformError> {
    Err(PlatformError::Unsupported(crate::Os::current()))
}

#[cfg(test)]
mod tests {
    use super::LocalTime;

    fn at(minute: u8, second: u8, millisecond: u16) -> LocalTime {
        LocalTime {
            year: 2026,
            month: 11,
            day: 7,
            weekday: 6,
            hour: 12,
            minute,
            second,
            millisecond,
        }
    }

    #[test]
    fn a_full_minute_remains_at_second_zero() {
        assert_eq!(at(34, 0, 0).millis_until_next_minute(), 60_000);
    }

    #[test]
    fn remaining_time_counts_seconds_and_milliseconds() {
        assert_eq!(at(34, 30, 500).millis_until_next_minute(), 29_500);
        assert_eq!(at(34, 59, 999).millis_until_next_minute(), 1);
    }

    #[test]
    fn remaining_time_is_never_zero_even_on_a_leap_second() {
        assert_eq!(at(34, 60, 0).millis_until_next_minute(), 1);
        assert_eq!(at(34, 59, 1000).millis_until_next_minute(), 1);
    }

    #[test]
    fn same_minute_ignores_seconds_but_not_minutes_or_days() {
        assert!(at(34, 0, 0).same_minute(&at(34, 59, 999)));
        assert!(!at(34, 59, 999).same_minute(&at(35, 0, 0)));

        let mut next_day = at(34, 0, 0);
        next_day.day = 8;
        assert!(!at(34, 0, 0).same_minute(&next_day));
    }

    #[cfg(not(windows))]
    #[test]
    fn local_time_is_unsupported_outside_windows() {
        assert!(matches!(
            super::local_time(),
            Err(crate::PlatformError::Unsupported(_))
        ));
    }

    #[cfg(windows)]
    #[test]
    fn local_time_is_in_range_on_windows() {
        let now = super::local_time().expect("Windows siempre devuelve la hora");
        assert!(now.year >= 2024);
        assert!((1..=12).contains(&now.month));
        assert!((1..=31).contains(&now.day));
        assert!(now.weekday < 7);
        assert!(now.hour < 24 && now.minute < 60 && now.second < 61);
        assert!(now.millisecond < 1000);
    }
}
