//! The cell colors the format panel offers.

use ely_lok::show::Paint;

/// A cell color: its state, command, colors, 0xRRGGBB, and clear.
pub(super) struct Pick {
    pub(super) id: &'static str,
    pub(super) state: &'static str,
    pub(super) command: &'static str,
    pub(super) colors: [u32; 10],
    pub(super) none: &'static str,
}

pub(super) const FILL: Pick = Pick {
    id: "table-fill",
    state: ".uno:BackgroundColor",
    command: "cells.fill",
    colors: [
        0xFFFFFF, 0xD9D9D9, 0xFFC7CE, 0xFFEB9C, 0xC6EFCE, 0xBDD7EE, 0xF8CBAD, 0xE4DFEC, 0x808080,
        0x000000,
    ],
    none: "table.format.no_fill",
};

/// Red marks, white reads on a dark fill (#15).
pub(super) const TEXT: Pick = Pick {
    id: "table-text-color",
    state: ".uno:Color",
    command: "cells.text_color",
    colors: [
        0x000000, 0xFFFFFF, 0xFF0000, 0xC00000, 0xFFC000, 0xFFFF00, 0x00B050, 0x0070C0, 0x7030A0,
        0x808080,
    ],
    none: "table.format.auto_color",
};

/// A color state as paint.
pub(crate) fn paint(state: Option<&str>) -> Paint {
    if state == Some(ely_lok::MIXED) {
        return Paint::Mixed;
    }
    state
        .and_then(|color| color.parse::<i64>().ok())
        .and_then(|color| u32::try_from(color).ok())
        .map_or(Paint::None, Paint::Color)
}
