//! The value box: type an exact size while a shape is being drawn, then press Enter.
//!
//! Characters are filtered as they arrive, so the box can only ever hold something that looks like
//! numbers: digits, a point, a minus, and a comma, semicolon or space between values.

/// Longest text the box accepts.
pub const MAX_LEN: usize = 32;

/// The text being typed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ValueBox {
    text: String,
}

impl ValueBox {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `c` when it can be part of a number list. Returns whether it was taken.
    pub fn push(&mut self, c: char) -> bool {
        let ok = c.is_ascii_digit() || matches!(c, '.' | '-' | ',' | ';' | ' ');
        if !ok || self.text.len() >= MAX_LEN {
            return false;
        }
        self.text.push(c);
        true
    }

    pub fn backspace(&mut self) {
        self.text.pop();
    }

    pub fn clear(&mut self) {
        self.text.clear();
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }

    pub fn is_empty(&self) -> bool {
        self.text.trim().is_empty()
    }

    /// The numbers typed, in order. `None` when the box is empty or anything in it is not a
    /// finite number.
    pub fn values(&self) -> Option<Vec<f64>> {
        let parts: Vec<&str> = self.text.split([',', ';', ' ']).filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            return None;
        }
        parts.iter().map(|s| s.parse::<f64>().ok().filter(|v| v.is_finite())).collect()
    }
}

/// A length for the readout: up to two decimals, no trailing zeros.
pub fn fmt_len(v: f64) -> String {
    if !v.is_finite() {
        return "–".to_string();
    }
    let s = format!("{v:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-0" { "0".to_string() } else { s.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(s: &str) -> ValueBox {
        let mut b = ValueBox::new();
        for c in s.chars() {
            b.push(c);
        }
        b
    }

    #[test]
    fn reads_one_or_two_numbers() {
        assert_eq!(typed("12.5").values(), Some(vec![12.5]));
        assert_eq!(typed("4,3").values(), Some(vec![4.0, 3.0]));
        assert_eq!(typed("4; 3").values(), Some(vec![4.0, 3.0]));
        assert_eq!(typed("-2 7").values(), Some(vec![-2.0, 7.0]));
    }

    #[test]
    fn letters_are_not_taken() {
        let mut b = ValueBox::new();
        assert!(!b.push('x'));
        assert!(!b.push('e'));
        assert!(b.is_empty());
    }

    #[test]
    fn nonsense_is_not_a_number() {
        assert_eq!(typed("1.2.3").values(), None);
        assert_eq!(typed("--5").values(), None);
        assert_eq!(typed("").values(), None);
        assert_eq!(typed(" , ").values(), None);
    }

    #[test]
    fn the_box_is_bounded() {
        let mut b = ValueBox::new();
        for _ in 0..100 {
            b.push('1');
        }
        assert_eq!(b.as_str().len(), MAX_LEN);
    }

    #[test]
    fn backspace_and_clear_work_on_an_empty_box() {
        let mut b = ValueBox::new();
        b.backspace();
        b.clear();
        assert!(b.is_empty());
    }

    #[test]
    fn lengths_read_cleanly() {
        assert_eq!(fmt_len(12.0), "12");
        assert_eq!(fmt_len(12.5), "12.5");
        assert_eq!(fmt_len(0.004), "0");
        assert_eq!(fmt_len(-0.001), "0");
        assert_eq!(fmt_len(f64::NAN), "–");
    }
}
