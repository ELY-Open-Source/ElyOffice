use super::*;

#[test]
fn the_kernel_gets_checked_lines_and_the_file_made_absolute() {
    let sent = request(&["a.xlsx", "S!A1:C1", "bold=on", "size=10.5", "background=#FFF2CC", "align=center"]).expect("a request");
    assert_eq!((sent[0].as_str(), sent[2].as_str()), ("range.format", "S!A1:C1"));
    assert!(std::path::Path::new(&sent[1]).is_absolute(), "{sent:?}");
    assert_eq!(sent[3], "align=center\nbackground=16773836\nbold=1\nsize=210\n");
    let lines = kernel_lines(&["color=none", "italic=off", "underline=on", "wrap=on", "font=Noto Sans", "number=#,##0.00", "valign=middle"]).expect("lines");
    assert_eq!(lines, "color=none\nfont=Noto Sans\nitalic=0\nnumber=#,##0.00\nunderline=1\nvalign=middle\nwrap=1\n");
    assert_eq!(number_code(&lines), "#,##0.00");
    assert!(request(&["a.xlsx"]).is_err(), "no address");
}

#[test]
fn borders_expand_and_a_named_edge_overrides() {
    let all = kernel_lines(&["border=all"]).expect("all");
    assert_eq!(all.lines().count(), 6, "{all}");
    assert!(all.lines().all(|line| line.ends_with("=0:15:0")), "{all}");
    let outer = kernel_lines(&["border=outer"]).expect("outer");
    assert!(!outer.contains("inner"), "outer leaves the inside: {outer}");
    let none = kernel_lines(&["border=none"]).expect("none");
    assert_eq!(none.matches("=none").count(), 6, "{none}");
    let mixed = kernel_lines(&["border-bottom=double:1.5pt:#000000", "border=outer"]).expect("mixed");
    assert!(mixed.contains("border-bottom=3:30:0\n") && mixed.contains("border-top=0:15:0\n"), "{mixed}");
    assert_eq!(edge("dash-dot:0.5mm:#FF0000"), Some("16:28:16711680".into()));
    assert_eq!(edge("solid:2mm:#000000"), Some("0:113:0".into()), "2 mm is allowed");
    for bad in ["solid:2.1mm:#000000", "solid:0pt:#000000", "wavy:1pt:#000000", "solid:1px:#000000", "solid:1pt", "solid:1pt:red", "solid:1pt:#000000:x", "solid:1pt:none"] {
        assert_eq!(edge(bad), None, "{bad}");
    }
}

#[test]
fn refusals_read_as_the_writer_wrote_them() {
    let refused = |pairs: &[&str]| kernel_lines(pairs).expect_err("refused");
    assert_eq!(refused(&[]), "no format properties given (use PROPERTY=VALUE)");
    assert_eq!(refused(&["shade=on"]), "shade: unknown format property");
    assert_eq!(refused(&["bold"]), "bold: expected PROPERTY=VALUE");
    assert_eq!(refused(&["font="]), "font=: expected PROPERTY=VALUE");
    assert_eq!(refused(&["bold=on", "bold=off"]), "bold: property given more than once");
    assert_eq!(refused(&["bold=yes"]), "bold: invalid value \"yes\" (expected on or off)");
    for size in ["0.5", "1000", "big", "NaN"] {
        assert_eq!(refused(&[&format!("size={size}")]), format!("size: invalid value \"{size}\" (must be a number from 1 to 999)"));
    }
    assert_eq!(refused(&["color=#FFF"]), "color: invalid color \"#FFF\" (expected #RRGGBB or none)");
    assert_eq!(refused(&["background=blue"]), "background: invalid color \"blue\" (expected #RRGGBB or none)");
    assert_eq!(refused(&["align=middle"]), "align: invalid value \"middle\" (expected left|center|right|justify|general)");
    assert_eq!(refused(&["valign=center"]), "valign: invalid value \"center\" (expected top|middle|bottom|standard)");
    assert_eq!(refused(&["border=box"]), "border: invalid border \"box\" (all|outer|none; side: none or style:width:#RRGGBB)");
    assert_eq!(refused(&["border-left=thick"]), "border-left: invalid border \"thick\" (all|outer|none; side: none or style:width:#RRGGBB)");
    assert_eq!(NUMBER.replace("{value}", "0.0.0\""), "number: invalid number format code \"0.0.0\"\"");
}
