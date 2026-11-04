use std::{ffi::c_char, ffi::c_void, io};

use super::{Callback, Doc, Document, cstring};

unsafe extern "C" {
    fn free(ptr: *mut c_void);
}

impl Document {
    /// Makes this document's view the kernel's current one.
    pub fn activate(&self) {
        let class = unsafe { (*self.doc).class };
        type F = unsafe extern "C" fn(*mut Doc, i32);
        call!(class, 27, F, self.doc, self.view);
    }

    /// Page count or area in LOK twips; zeros when unset.
    pub fn size(&self) -> (i64, i64) {
        let class = unsafe { (*self.doc).class };
        type F = unsafe extern "C" fn(*mut Doc, *mut i64, *mut i64);
        let (mut width, mut height) = (0i64, 0i64);
        call!(class, 11, F, self.doc, &mut width, &mut height);
        (width, height)
    }

    /// Paints the document into `buffer` at (x, y) twips.
    pub fn paint_rect(&self, buffer: &mut [u8], pixels: (i32, i32), twips: (i32, i32, i32, i32)) {
        let class = unsafe { (*self.doc).class };
        type F = unsafe extern "C" fn(*mut Doc, *mut u8, i32, i32, i32, i32, i32, i32);
        let (pw, ph) = pixels;
        let (x, y, w, h) = twips;
        call!(
            class,
            9,
            F,
            self.doc,
            buffer.as_mut_ptr(),
            pw,
            ph,
            x,
            y,
            w,
            h
        );
    }

    /// Paints part `part` whole; `mode` 1 paints masters.
    pub fn paint_part(&self, buffer: &mut [u8], (part, mode): (i32, i32), pixels: (i32, i32)) {
        let (w, h) = self.size();
        let class = unsafe { (*self.doc).class };
        type F = unsafe extern "C" fn(*mut Doc, *mut u8, i32, i32, i32, i32, i32, i32, i32, i32);
        call!(
            class,
            32,
            F,
            self.doc,
            buffer.as_mut_ptr(),
            part,
            mode,
            pixels.0,
            pixels.1,
            0,
            0,
            w as i32,
            h as i32
        );
    }

    /// Initializes rendering so callbacks flow (LOK-03).
    pub fn initialize_for_rendering(&self) {
        let class = unsafe { (*self.doc).class };
        type F = unsafe extern "C" fn(*mut Doc, *const c_char);
        call!(class, 12, F, self.doc, std::ptr::null());
    }

    /// Registers `callback` with `data` (LOK-03).
    pub fn set_callback(&self, callback: Option<Callback>, data: *mut c_void) {
        let class = unsafe { (*self.doc).class };
        type F = unsafe extern "C" fn(*mut Doc, Option<Callback>, *mut c_void);
        call!(class, 13, F, self.doc, callback, data);
    }

    /// Runs a UNO command with JSON `args` (LOK-08).
    pub fn post_uno(&self, command: &str, args: Option<&str>) -> io::Result<()> {
        self.dispatch(command, args, false)
    }

    /// Runs a UNO command and asks for its completion result.
    pub fn post_uno_notify(&self, command: &str, args: Option<&str>) -> io::Result<()> {
        self.dispatch(command, args, true)
    }

    fn dispatch(&self, command: &str, args: Option<&str>, notify: bool) -> io::Result<()> {
        let class = unsafe { (*self.doc).class };
        type F = unsafe extern "C" fn(*mut Doc, *const c_char, *const c_char, bool);
        let command = cstring(command)?;
        let args = args.map(cstring).transpose()?;
        let args = args.as_ref().map_or(std::ptr::null(), |args| args.as_ptr());
        call!(class, 16, F, self.doc, command.as_ptr(), args, notify);
        Ok(())
    }

    /// Posts a key event: 0 input, 1 up.
    pub fn post_key(&self, event_type: i32, char_code: i32, key_code: i32) {
        let class = unsafe { (*self.doc).class };
        type F = unsafe extern "C" fn(*mut Doc, i32, i32, i32);
        call!(class, 14, F, self.doc, event_type, char_code, key_code);
    }

    /// Posts a mouse event: 0 down, 1 up, 2 move.
    pub fn post_mouse(&self, event: [i32; 6]) {
        let class = unsafe { (*self.doc).class };
        type F = unsafe extern "C" fn(*mut Doc, i32, i32, i32, i32, i32, i32);
        let [kind, x, y, count, buttons, modifier] = event;
        call!(class, 15, F, self.doc, kind, x, y, count, buttons, modifier);
    }

    /// Moves a selected object's handle: 0 start, 1 end.
    pub fn set_graphic_selection(&self, kind: i32, x: i32, y: i32) {
        let class = unsafe { (*self.doc).class };
        type F = unsafe extern "C" fn(*mut Doc, i32, i32, i32);
        call!(class, 20, F, self.doc, kind, x, y);
    }

    /// Posts IME text: 0 composing, 2 commit (LOK-07).
    pub fn post_text(&self, event_type: i32, text: &str) -> io::Result<()> {
        let class = unsafe { (*self.doc).class };
        type F = unsafe extern "C" fn(*mut Doc, u32, i32, *const c_char);
        let text = cstring(text)?;
        call!(class, 40, F, self.doc, 0, event_type, text.as_ptr());
        Ok(())
    }

    /// LOK_DOCTYPE: 0 text, 1 spreadsheet, 2 presentation.
    pub fn kind(&self) -> i32 {
        let class = unsafe { (*self.doc).class };
        type F = unsafe extern "C" fn(*mut Doc) -> i32;
        call!(class, 2, F, self.doc)
    }

    /// LOK_SELTYPE: 0 none, 1 text, 3 complex.
    pub fn selection_type(&self) -> i32 {
        let class = unsafe { (*self.doc).class };
        type F = unsafe extern "C" fn(*mut Doc) -> i32;
        call!(class, 54, F, self.doc)
    }

    /// The view clipboard in `mimes`; empty ones skipped (LOK-06).
    pub fn clipboard(&self, mimes: &[&str]) -> io::Result<Vec<(String, Vec<u8>)>> {
        let class = unsafe { (*self.doc).class };
        type F = unsafe extern "C" fn(
            *mut Doc,
            *const *const c_char,
            *mut usize,
            *mut *mut *mut c_char,
            *mut *mut usize,
            *mut *mut *mut c_char,
        ) -> i32;
        let owned = mimes
            .iter()
            .map(|mime| cstring(mime))
            .collect::<io::Result<Vec<_>>>()?;
        let mut list: Vec<*const c_char> = owned.iter().map(|mime| mime.as_ptr()).collect();
        list.push(std::ptr::null());
        let (mut count, mut names, mut sizes, mut streams) = (
            0usize,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        let ok = call!(
            class,
            52,
            F,
            self.doc,
            list.as_ptr(),
            &mut count,
            &mut names,
            &mut sizes,
            &mut streams
        );
        if ok == 0 {
            return Err(io::Error::other("getClipboard failed"));
        }
        let mut found = Vec::new();
        for i in 0..count {
            let (name, size, stream) = unsafe { (*names.add(i), *sizes.add(i), *streams.add(i)) };
            if size > 0 && !stream.is_null() {
                let mime = unsafe { std::ffi::CStr::from_ptr(name) };
                let data = unsafe { std::slice::from_raw_parts(stream.cast::<u8>(), size) };
                found.push((mime.to_string_lossy().into_owned(), data.to_vec()));
            }
            unsafe {
                free(name.cast());
                free(stream.cast());
            }
        }
        unsafe {
            free(names.cast());
            free(sizes.cast());
            free(streams.cast());
        }
        Ok(found)
    }

    /// The JSON LOK keeps for `command` (getCommandValues).
    pub fn command_values(&self, command: &str) -> io::Result<String> {
        let class = unsafe { (*self.doc).class };
        type F = unsafe extern "C" fn(*mut Doc, *const c_char) -> *mut c_char;
        let name = cstring(command)?;
        let raw = call!(class, 22, F, self.doc, name.as_ptr());
        if raw.is_null() {
            return Err(io::Error::other(format!("no values for {command}")));
        }
        let text = unsafe { std::ffi::CStr::from_ptr(raw) }
            .to_string_lossy()
            .into_owned();
        unsafe { free(raw.cast()) };
        Ok(text)
    }

    /// Turns on the a11y callbacks for the current view.
    pub fn enable_a11y(&self) {
        let class = unsafe { (*self.doc).class };
        type View = unsafe extern "C" fn(*mut Doc) -> i32;
        type F = unsafe extern "C" fn(*mut Doc, i32, bool);
        let view = call!(class, 28, View, self.doc);
        call!(class, 69, F, self.doc, view, true);
    }

    /// Tells the kernel a tile's size in pixels and twips.
    pub fn set_client_zoom(&self, pixels: i32, twips: i32) {
        let class = unsafe { (*self.doc).class };
        type F = unsafe extern "C" fn(*mut Doc, i32, i32, i32, i32);
        call!(class, 23, F, self.doc, pixels, pixels, twips, twips);
    }

    /// Tells the kernel the area the client shows, in twips.
    pub fn set_visible_area(&self, x: i32, y: i32, width: i32, height: i32) {
        let class = unsafe { (*self.doc).class };
        type F = unsafe extern "C" fn(*mut Doc, i32, i32, i32, i32);
        call!(class, 24, F, self.doc, x, y, width, height);
    }

    /// The selection as plain text; Calc joins cells by tabs.
    pub fn selection_text(&self) -> io::Result<String> {
        let class = unsafe { (*self.doc).class };
        type F = unsafe extern "C" fn(*mut Doc, *const c_char, *mut *mut c_char) -> *mut c_char;
        let mime = cstring("text/plain;charset=utf-8")?;
        let mut used: *mut c_char = std::ptr::null_mut();
        let text = owned(call!(class, 18, F, self.doc, mime.as_ptr(), &mut used));
        owned(used);
        text.ok_or_else(|| io::Error::other("the kernel gave no selection text"))
    }

    /// Completes the function name being typed (TABLE-03).
    pub fn complete_function(&self, name: &str) -> io::Result<()> {
        let class = unsafe { (*self.doc).class };
        type F = unsafe extern "C" fn(*mut Doc, *const c_char);
        let name = cstring(name)?;
        call!(class, 59, F, self.doc, name.as_ptr());
        Ok(())
    }

    /// How many parts: sheets or slides.
    pub fn parts(&self) -> i32 {
        let class = unsafe { (*self.doc).class };
        type F = unsafe extern "C" fn(*mut Doc) -> i32;
        call!(class, 3, F, self.doc)
    }

    /// The part the view shows, from 0.
    pub fn part(&self) -> i32 {
        let class = unsafe { (*self.doc).class };
        type F = unsafe extern "C" fn(*mut Doc) -> i32;
        call!(class, 5, F, self.doc)
    }

    /// The view's edit mode: 1 on an Impress master.
    pub fn edit_mode(&self) -> i32 {
        let class = unsafe { (*self.doc).class };
        type F = unsafe extern "C" fn(*mut Doc) -> i32;
        call!(class, 67, F, self.doc)
    }

    /// Shows part `part`.
    pub fn set_part(&self, part: i32) {
        let class = unsafe { (*self.doc).class };
        type F = unsafe extern "C" fn(*mut Doc, i32);
        call!(class, 6, F, self.doc, part);
    }

    /// The name of part `part`.
    pub fn part_name(&self, part: i32) -> io::Result<String> {
        let class = unsafe { (*self.doc).class };
        type F = unsafe extern "C" fn(*mut Doc, i32) -> *mut c_char;
        owned(call!(class, 7, F, self.doc, part))
            .ok_or_else(|| io::Error::other(format!("part {part} has no name")))
    }

    /// LOK's JSON about part `part`: visible, protected.
    pub fn part_info(&self, part: i32) -> io::Result<String> {
        let class = unsafe { (*self.doc).class };
        type F = unsafe extern "C" fn(*mut Doc, i32) -> *mut c_char;
        owned(call!(class, 41, F, self.doc, part))
            .ok_or_else(|| io::Error::other(format!("part {part} has no info")))
    }

    /// Page rectangles as "x, y, w, h; ...".
    pub fn page_rects(&self) -> io::Result<String> {
        let class = unsafe { (*self.doc).class };
        type F = unsafe extern "C" fn(*mut Doc) -> *mut c_char;
        let raw = call!(class, 4, F, self.doc);
        if raw.is_null() {
            return Ok(String::new());
        }
        let text = unsafe { std::ffi::CStr::from_ptr(raw) }
            .to_string_lossy()
            .into_owned();
        unsafe { free(raw.cast()) };
        Ok(text)
    }

    /// The focused paragraph as LOK's a11y JSON.
    pub fn a11y_paragraph(&self) -> io::Result<String> {
        let class = unsafe { (*self.doc).class };
        type F = unsafe extern "C" fn(*mut Doc) -> *mut c_char;
        let raw = call!(class, 70, F, self.doc);
        if raw.is_null() {
            return Err(io::Error::other("no focused paragraph"));
        }
        let text = unsafe { std::ffi::CStr::from_ptr(raw) }
            .to_string_lossy()
            .into_owned();
        unsafe { free(raw.cast()) };
        Ok(text)
    }

    /// Puts one `mime` entry on the view clipboard (LOK-06).
    pub fn set_clipboard(&self, mime: &str, data: &[u8]) -> io::Result<()> {
        let class = unsafe { (*self.doc).class };
        type F = unsafe extern "C" fn(
            *mut Doc,
            usize,
            *const *const c_char,
            *const usize,
            *const *const c_char,
        ) -> i32;
        let mime = cstring(mime)?;
        let ok = call!(
            class,
            53,
            F,
            self.doc,
            1,
            &mime.as_ptr(),
            &data.len(),
            &data.as_ptr().cast::<c_char>()
        );
        if ok == 0 {
            return Err(io::Error::other("setClipboard failed"));
        }
        Ok(())
    }

    /// Saves to `url` as `format`; false means LOK reported failure.
    pub fn save_as(&self, url: &str, format: &str, options: &str) -> io::Result<()> {
        let class = unsafe { (*self.doc).class };
        let url_c = cstring(url)?;
        let format_c = cstring(format)?;
        let options_c = cstring(options)?;
        type F = unsafe extern "C" fn(*mut Doc, *const i8, *const i8, *const i8) -> i32;
        let ok = call!(
            class,
            1,
            F,
            self.doc,
            url_c.as_ptr(),
            format_c.as_ptr(),
            options_c.as_ptr()
        );
        if ok == 0 {
            return Err(io::Error::other(format!("saveAs failed for {url}")));
        }
        Ok(())
    }
}

impl Drop for Document {
    fn drop(&mut self) {
        if self.doc.is_null() {
            return;
        }
        let class = unsafe { (*self.doc).class };
        type F = unsafe extern "C" fn(*mut Doc);
        call!(class, 0, F, self.doc);
    }
}

/// A document's native handle, for callbacks that cannot borrow it.
#[derive(Clone, Copy)]
pub struct Raw(*mut Doc);

// SAFETY: dereferenced only on the kernel's thread.
unsafe impl Send for Raw {}

impl Document {
    pub fn raw(&self) -> Raw {
        Raw(self.doc)
    }
}

/// Closes kernel window `window` of document `doc` (LOK-08).
pub fn close_window(doc: Raw, window: u32) {
    let doc = doc.0;
    let class = unsafe { (*doc).class };
    type F = unsafe extern "C" fn(*mut Doc, u32, i32, *const c_char);
    call!(class, 36, F, doc, window, 0, std::ptr::null());
}

/// Takes a string LOK allocated; None for null.
fn owned(raw: *mut c_char) -> Option<String> {
    if raw.is_null() {
        return None;
    }
    let text = unsafe { std::ffi::CStr::from_ptr(raw) }
        .to_string_lossy()
        .into_owned();
    unsafe { free(raw.cast()) };
    Some(text)
}
