//! Small PDFs written by hand, for tests.

/// A bookmark: title, page, and those under it.
pub type Mark<'a> = (&'a str, usize, &'a [(&'a str, usize)]);

/// Links, form fields and page labels to add.
#[derive(Default)]
pub struct Extras<'a> {
    /// Link annotations: from page, to page.
    pub links: &'a [(usize, usize)],
    /// A text field on each of these pages.
    pub fields: &'a [usize],
    /// Page labels: first page, style name.
    pub labels: &'a [(usize, &'a str)],
    /// Fields leave out their optional /P.
    pub loose: bool,
    /// Every page's /UserUnit, if not one.
    pub unit: Option<f32>,
    /// With `unit`: on /Pages, not each page.
    pub inherited: bool,
    /// Field names start with this, else "field".
    pub prefix: Option<&'a str>,
    /// The form's look: red 24 pt; font key, DA token.
    pub look: Option<(&'a str, &'a str)>,
    /// The form's default alignment, /Q.
    pub align: Option<u8>,
    /// Page one: a form of this text at 300, 400.
    pub stamp: Option<&'a str>,
    /// Page one: a 2x2 image, 100x50 at 72, 500.
    pub picture: bool,
    /// With `picture`: page one draws it full-page first.
    pub backdrop: bool,
    /// Fields hang under one parent of this name.
    pub parent: Option<&'a str>,
    /// A parent and child field with no widget.
    pub bare: Option<(&'a str, &'a str)>,
    /// Page one: a file annotation, name and bytes, no /Size.
    pub attached: Option<(&'a str, &'a [u8])>,
}

/// A PDF of `pages` (width, height, lines), with bookmarks.
pub fn pdf(pages: &[(f32, f32, &[&str])], marks: &[Mark]) -> Vec<u8> {
    pdf_with(pages, marks, &Extras::default())
}

/// A PDF as `pdf` makes, with `extras`.
pub fn pdf_with(pages: &[(f32, f32, &[&str])], marks: &[Mark], extras: &Extras) -> Vec<u8> {
    let mut objs: Vec<Vec<u8>> = Vec::new();
    let page_ids: Vec<usize> = (0..pages.len()).map(|n| 4 + n * 2).collect();
    // Links, then fields, after the outline objects.
    let outline_objs: usize = marks.iter().map(|(_, _, down)| 1 + down.len()).sum();
    let first_extra = 4 + pages.len() * 2 + 1 + outline_objs;
    let field_ids: Vec<usize> = (0..extras.fields.len()).map(|n| first_extra + extras.links.len() + n).collect();
    let mut catalog = String::from("<< /Type /Catalog /Pages 2 0 R /Outlines 3 0 R");
    // The parent field comes last of all.
    let parent_id = first_extra + extras.links.len() + extras.fields.len() + usize::from(extras.stamp.is_some()) + usize::from(extras.picture);
    let bare_id = parent_id + usize::from(extras.parent.is_some());
    let attach_id = bare_id + 2 * usize::from(extras.bare.is_some());
    if !field_ids.is_empty() || extras.bare.is_some() {
        let mut refs: Vec<String> = match extras.parent {
            Some(_) => vec![format!("{parent_id} 0 R")],
            None => field_ids.iter().map(|id| format!("{id} 0 R")).collect(),
        };
        refs.extend(extras.bare.map(|_| format!("{bare_id} 0 R")));
        let font = 4 + pages.len() * 2;
        let look = extras.look.map_or_else(String::new, |(key, token)| format!(" /DA (/{token} 24 Tf 1 0 0 rg) /DR << /Font << /{key} {font} 0 R >> >>"));
        let align = extras.align.map_or_else(String::new, |q| format!(" /Q {q}"));
        catalog.push_str(&format!(" /AcroForm << /Fields [{}]{look}{align} >>", refs.join(" ")));
    }
    if !extras.labels.is_empty() {
        let nums: Vec<String> = extras.labels.iter().map(|(at, style)| format!("{at} << /S /{style} >>")).collect();
        catalog.push_str(&format!(" /PageLabels << /Nums [{}] >>", nums.join(" ")));
    }
    objs.push(format!("{catalog} >>").into_bytes());
    let kids: Vec<String> = page_ids.iter().map(|id| format!("{id} 0 R")).collect();
    let (on_pages, on_page) = match extras.unit {
        Some(unit) if extras.inherited => (format!(" /UserUnit {unit}"), String::new()),
        Some(unit) => (String::new(), format!(" /UserUnit {unit}")),
        None => (String::new(), String::new()),
    };
    objs.push(
        format!(
            "<< /Type /Pages /Kids [{}] /Count {}{on_pages} >>",
            kids.join(" "),
            pages.len()
        )
        .into_bytes(),
    );
    // Outline objects follow the pages.
    let mut id = 4 + pages.len() * 2 + 1;
    let tops: Vec<usize> = (marks.iter())
        .map(|(_, _, down)| {
            let top = id;
            id += 1 + down.len();
            top
        })
        .collect();
    objs.push(match (tops.first(), tops.last()) {
        (Some(first), Some(last)) => format!(
            "<< /Type /Outlines /First {first} 0 R /Last {last} 0 R /Count {} >>",
            tops.len()
        )
        .into_bytes(),
        _ => b"<< /Type /Outlines /Count 0 >>".to_vec(),
    });
    for (n, (w, h, lines)) in pages.iter().enumerate() {
        let mut stream = match n == 0 && extras.backdrop {
            true => format!("q {w} 0 0 {h} 0 0 cm /Im1 Do Q\n"),
            false => String::new(),
        };
        stream.push_str("BT /F1 18 Tf 72 0 Td\n");
        for (at, line) in lines.iter().enumerate() {
            let y = h - 72.0 - at as f32 * 24.0;
            stream.push_str(&format!("1 0 0 1 72 {y} Tm ({line}) Tj\n"));
        }
        stream.push_str("ET");
        let font = 4 + pages.len() * 2;
        let (drawn, xobjects) = match n {
            0 => xobjects(extras, first_extra + extras.links.len() + extras.fields.len()),
            _ => (String::new(), String::new()),
        };
        stream.push_str(&drawn);
        let links = (extras.links.iter().enumerate()).filter(|(_, (from, _))| *from == n).map(|(at, _)| first_extra + at);
        let fields = (extras.fields.iter().enumerate()).filter(|(_, page)| **page == n).map(|(at, _)| field_ids[at]);
        let attached = extras.attached.filter(|_| n == 0).map(|_| attach_id);
        let annots: Vec<String> = links.chain(fields).chain(attached).map(|id| format!("{id} 0 R")).collect();
        objs.push(format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {w} {h}]{on_page} /Contents {} 0 R /Resources << /Font << /F1 {font} 0 R >>{xobjects} >> /Annots [{}] >>", page_ids[n] + 1, annots.join(" ")).into_bytes());
        objs.push(
            format!(
                "<< /Length {} >>\nstream\n{stream}\nendstream",
                stream.len()
            )
            .into_bytes(),
        );
    }
    objs.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec());
    // Top marks link to each other; children hang under them.
    for (at, (title, page, down)) in marks.iter().enumerate() {
        let top = tops[at];
        let mut dict = format!(
            "<< /Title ({title}) /Parent 3 0 R /Dest [{} 0 R /XYZ 0 700 0]",
            page_ids[*page]
        );
        if at > 0 {
            dict.push_str(&format!(" /Prev {} 0 R", tops[at - 1]));
        }
        if at + 1 < tops.len() {
            dict.push_str(&format!(" /Next {} 0 R", tops[at + 1]));
        }
        if !down.is_empty() {
            dict.push_str(&format!(
                " /First {} 0 R /Last {} 0 R /Count {}",
                top + 1,
                top + down.len(),
                down.len()
            ));
        }
        objs.push(format!("{dict} >>").into_bytes());
        for (n, (title, page)) in down.iter().enumerate() {
            let mut dict = format!(
                "<< /Title ({title}) /Parent {top} 0 R /Dest [{} 0 R /XYZ 0 500 0]",
                page_ids[*page]
            );
            if n > 0 {
                dict.push_str(&format!(" /Prev {} 0 R", top + n));
            }
            if n + 1 < down.len() {
                dict.push_str(&format!(" /Next {} 0 R", top + n + 2));
            }
            objs.push(format!("{dict} >>").into_bytes());
        }
    }
    for (_, to) in extras.links {
        objs.push(format!("<< /Type /Annot /Subtype /Link /Rect [72 72 200 100] /Border [0 0 0] /Dest [{} 0 R /Fit] >>", page_ids[*to]).into_bytes());
    }
    for (n, page) in extras.fields.iter().enumerate() {
        let owner = if extras.loose { String::new() } else { format!(" /P {} 0 R", page_ids[*page]) };
        let name = extras.prefix.unwrap_or("field");
        let parent = extras.parent.map_or_else(String::new, |_| format!(" /Parent {parent_id} 0 R"));
        objs.push(format!("<< /Type /Annot /Subtype /Widget /FT /Tx /T ({name}{n}) /V (value{n}) /Rect [72 120 300 140]{owner}{parent} >>").into_bytes());
    }
    if let Some(text) = extras.stamp {
        let font = 4 + pages.len() * 2;
        let content = format!("BT /F1 12 Tf 0 10 Td ({text}) Tj ET");
        objs.push(format!("<< /Type /XObject /Subtype /Form /BBox [0 0 200 40] /Resources << /Font << /F1 {font} 0 R >> >> /Length {} >>\nstream\n{content}\nendstream", content.len()).into_bytes());
    }
    if extras.picture {
        let mut image = b"<< /Type /XObject /Subtype /Image /Width 2 /Height 2 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Length 12 >>\nstream\n".to_vec();
        image.extend_from_slice(&[255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255]);
        image.extend_from_slice(b"\nendstream");
        objs.push(image);
    }
    if let Some(name) = extras.parent {
        let kids: Vec<String> = field_ids.iter().map(|id| format!("{id} 0 R")).collect();
        objs.push(format!("<< /T ({name}) /Kids [{}] >>", kids.join(" ")).into_bytes());
    }
    if let Some((parent, child)) = extras.bare {
        objs.push(format!("<< /T ({parent}) /Kids [{} 0 R] >>", bare_id + 1).into_bytes());
        objs.push(format!("<< /T ({child}) /FT /Tx /V (Alice) /Parent {bare_id} 0 R >>").into_bytes());
    }
    if let Some((name, bytes)) = extras.attached {
        objs.push(format!("<< /Type /Annot /Subtype /FileAttachment /Rect [72 72 90 90] /FS {} 0 R >>", attach_id + 1).into_bytes());
        objs.push(format!("<< /Type /Filespec /F ({name}) /UF ({name}) /EF << /F {} 0 R >> >>", attach_id + 2).into_bytes());
        let mut file = format!("<< /Type /EmbeddedFile /Length {} >>\nstream\n", bytes.len()).into_bytes();
        file.extend_from_slice(bytes);
        file.extend_from_slice(b"\nendstream");
        objs.push(file);
    }
    file(&objs)
}

/// Page one's form and image draws, and their resources.
fn xobjects(extras: &Extras, first: usize) -> (String, String) {
    let (mut drawn, mut names, mut id) = (String::new(), String::new(), first);
    if extras.stamp.is_some() {
        drawn.push_str("\nq 1 0 0 1 300 400 cm /Fm1 Do Q");
        names.push_str(&format!(" /Fm1 {id} 0 R"));
        id += 1;
    }
    if extras.picture {
        drawn.push_str("\nq 100 0 0 50 72 500 cm /Im1 Do Q");
        names.push_str(&format!(" /Im1 {id} 0 R"));
    }
    match names.is_empty() {
        true => (drawn, names),
        false => (drawn, format!(" /XObject <<{names} >>")),
    }
}

/// One page of `size` points: a gray picture only.
pub fn scan((w, h): (f32, f32), (pw, ph): (u32, u32), gray: &[u8]) -> Vec<u8> {
    let content = format!("q {w} 0 0 {h} 0 0 cm /Im1 Do Q");
    let mut image = format!("<< /Type /XObject /Subtype /Image /Width {pw} /Height {ph} /ColorSpace /DeviceGray /BitsPerComponent 8 /Length {} >>\nstream\n", gray.len()).into_bytes();
    image.extend_from_slice(gray);
    image.extend_from_slice(b"\nendstream");
    let objs: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {w} {h}] /Contents 4 0 R /Resources << /XObject << /Im1 5 0 R >> >> >>").into_bytes(),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()).into_bytes(),
        image,
    ];
    file(&objs)
}

/// Numbered objects as a whole PDF file.
fn file(objs: &[Vec<u8>]) -> Vec<u8> {
    let mut body = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (n, obj) in objs.iter().enumerate() {
        offsets.push(body.len());
        body.extend_from_slice(format!("{} 0 obj\n", n + 1).as_bytes());
        body.extend_from_slice(obj);
        body.extend_from_slice(b"\nendobj\n");
    }
    let xref = body.len();
    body.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes());
    for offset in offsets {
        body.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    body.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objs.len() + 1).as_bytes());
    body
}

/// One Letter page drawing `content`; `objects` are numbered from 5.
pub fn one_page(catalog: &str, page: &str, content: &str, objects: &[String]) -> Vec<u8> {
    let mut objs: Vec<Vec<u8>> = vec![
        format!("<< /Type /Catalog /Pages 2 0 R{catalog} >>").into_bytes(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R{page} >>").into_bytes(),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()).into_bytes(),
    ];
    objs.extend(objects.iter().map(|obj| obj.clone().into_bytes()));
    file(&objs)
}
