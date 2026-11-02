//! The LOK_CALLBACK kinds Ely reads.

pub(super) const INVALIDATE_TILES: i32 = 0;
pub(super) const CURSOR: i32 = 1;
pub(super) const SELECTION: i32 = 2;
pub(super) const CURSOR_VISIBLE: i32 = 5;
pub(super) const GRAPHIC: i32 = 6;
pub(super) const HYPERLINK: i32 = 7;
pub(super) const STATE: i32 = 8;
pub(super) const RULER: i32 = 35;
pub(super) const WINDOW: i32 = 36;
pub(super) const JSDIALOG: i32 = 46;
pub(super) const RESULT: i32 = 16;
pub(super) const A11Y_FOCUS: i32 = 62;
pub(super) const A11Y_CARET: i32 = 63;
pub(super) const A11Y_SELECTION: i32 = 64;
pub(super) const A11Y_CELL: i32 = 67;
pub(super) const A11Y_EDITING: i32 = 68;
pub(super) const A11Y_OBJECT: i32 = 69;
pub(super) const REDLINES: i32 = 30;
pub(super) const REDLINE: i32 = 31;
pub(super) const COMMENT: i32 = 32;
pub(super) const NOT_FOUND: i32 = 12;
pub(super) const FOUND: i32 = 15;
pub(super) const MENU: i32 = 23;
pub(super) const FONTS: i32 = 57;
pub(super) const SIZE_CHANGED: i32 = 13;
pub(super) const SET_PART: i32 = 14;
pub(super) const CELL_CURSOR: i32 = 17;
pub(super) const MOUSE_POINTER: i32 = 18;
pub(super) const CELL_FORMULA: i32 = 19;
pub(super) const HEADER: i32 = 33;
pub(super) const GEOMETRY: i32 = 50;
pub(super) const CELL_ADDRESS: i32 = 34;
pub(super) const REFERENCE_MARKS: i32 = 45;
pub(super) const FUNCTION_LIST: i32 = 47;
pub(super) const TOOLTIP: i32 = 71;
pub(super) const TABLE: i32 = 44;

pub(super) const KINDS: [i32; 36] = [
    TABLE,
    MOUSE_POINTER,
    REFERENCE_MARKS,
    FUNCTION_LIST,
    TOOLTIP,
    GEOMETRY,
    SIZE_CHANGED,
    SET_PART,
    CELL_CURSOR,
    CELL_FORMULA,
    HEADER,
    CELL_ADDRESS,
    FONTS,
    NOT_FOUND,
    FOUND,
    MENU,
    REDLINES,
    REDLINE,
    COMMENT,
    RULER,
    INVALIDATE_TILES,
    CURSOR,
    SELECTION,
    CURSOR_VISIBLE,
    GRAPHIC,
    HYPERLINK,
    STATE,
    RESULT,
    WINDOW,
    JSDIALOG,
    A11Y_FOCUS,
    A11Y_CARET,
    A11Y_SELECTION,
    A11Y_CELL,
    A11Y_EDITING,
    A11Y_OBJECT,
];
