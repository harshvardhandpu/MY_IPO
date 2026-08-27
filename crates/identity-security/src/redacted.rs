//! A reusable wrapper for values that must never surface in logs, errors,
//! debug output, telemetry, or AI payloads.

use zeroize::Zeroize;

/// Wraps a sensitive value so any accidental `Display`/`Debug` renders a fixed
/// `[REDACTED]` marker. The underlying value is zeroized on drop and reachable
/// only through the explicitly named [`Redacted::expose`] accessor.
pub struct Redacted<T: Zeroize>(T);

impl<T: Zeroize> Redacted<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }

    /// Deliberately reveal the wrapped value. Callers must justify this use.
    pub fn expose(&self) -> &T {
        &self.0
    }
}

impl<T: Zeroize> std::fmt::Display for Redacted<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl<T: Zeroize> std::fmt::Debug for Redacted<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl<T: Zeroize> Drop for Redacted<T> {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacted_renders_fixed_marker() {
        let r = Redacted::new("hunter2".to_string());
        assert_eq!(format!("{r}"), "[REDACTED]");
        assert_eq!(format!("{r:?}"), "[REDACTED]");
        assert_eq!(r.expose(), "hunter2");
    }
}
