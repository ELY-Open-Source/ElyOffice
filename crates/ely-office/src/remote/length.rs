//! Lengths such as `2.5cm`, shared by ElyTable and ElyDoc.

/// Twips per unit a length may be written in.
const UNITS: [(&str, f64); 4] = [("cm", 1440.0 / 2.54), ("mm", 144.0 / 2.54), ("in", 1440.0), ("pt", 20.0)];
/// From 1 pt to LibreOffice's 1 m, as it rounds.
const LENGTHS: (u32, u32) = (20, 56693);

/// Why a length was refused.
#[derive(Debug, PartialEq, Eq)]
pub enum Refused {
    /// Not a number followed by cm, mm, in or pt.
    Form,
    /// Outside 1pt to 100cm.
    Range,
}

/// `size` in twips.
pub fn twips(size: &str) -> Result<u32, Refused> {
    let (number, per) = UNITS.iter().find_map(|(unit, per)| size.strip_suffix(unit).map(|number| (number, per))).ok_or(Refused::Form)?;
    let amount: f64 = number.parse().map_err(|_| Refused::Form)?;
    let twips = (amount * per).round();
    if !twips.is_finite() || twips < f64::from(LENGTHS.0) || twips > f64::from(LENGTHS.1) {
        return Err(Refused::Range);
    }
    // In range, so the cast keeps the value.
    Ok(twips as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_number_and_unit_within_1pt_and_100cm() {
        assert_eq!(twips("2.54cm"), Ok(1440));
        assert_eq!(twips("10mm"), Ok(567));
        assert_eq!((twips("1in"), twips("12pt")), (Ok(1440), Ok(240)));
        assert_eq!((twips("1pt"), twips("100cm")), (Ok(20), Ok(56693)), "both ends");
        for out in ["0.5pt", "100.01cm", "-1cm", "NaNcm"] {
            assert_eq!(twips(out), Err(Refused::Range), "{out}");
        }
        for bad in ["8", "8px", "cm", "1,5cm", ""] {
            assert_eq!(twips(bad), Err(Refused::Form), "{bad}");
        }
    }
}
