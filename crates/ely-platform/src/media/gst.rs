use std::{
    cell::Cell,
    ffi::{CStr, CString, c_char, c_int, c_uint, c_void},
    os::unix::ffi::OsStrExt,
    path::Path,
    sync::OnceLock,
};

use super::{Frame, State};

type Gboolean = c_int;
type Object = *mut c_void;

const FORMAT_TIME: c_int = 3;
const STATE_NULL: c_int = 1;
const STATE_PAUSED: c_int = 3;
const STATE_PLAYING: c_int = 4;
const CHANGE_FAILURE: c_int = 0;
const SEEK_FLUSH_ACCURATE: c_int = 1 | 2;
const MESSAGE_EOS: c_uint = 1;
const MESSAGE_ERROR: c_uint = 1 << 1;
const MESSAGE_ELEMENT: c_uint = 1 << 15;
const MAP_READ: c_int = 1;

#[repr(C)]
struct MiniObject {
    gtype: usize,
    refcount: c_int,
    lockstate: c_int,
    flags: c_uint,
    copy: *mut c_void,
    dispose: *mut c_void,
    free: *mut c_void,
    priv_uint: c_uint,
    priv_pointer: *mut c_void,
}

#[repr(C)]
struct Message {
    mini: MiniObject,
    kind: c_uint,
}

#[repr(C)]
struct GError {
    domain: u32,
    code: c_int,
    message: *mut c_char,
}

#[repr(C)]
struct MapInfo {
    memory: *mut c_void,
    flags: c_int,
    data: *mut u8,
    size: usize,
    maxsize: usize,
    user_data: [*mut c_void; 4],
    reserved: [*mut c_void; 4],
}

/// The calls taken from the system's GStreamer.
struct Api {
    init_check: unsafe extern "C" fn(*mut c_int, *mut c_void, *mut *mut GError) -> Gboolean,
    make: unsafe extern "C" fn(*const c_char, *const c_char) -> Object,
    set: unsafe extern "C" fn(Object, *const c_char, ...),
    get: unsafe extern "C" fn(Object, *const c_char, ...),
    caps_from_string: unsafe extern "C" fn(*const c_char) -> Object,
    set_state: unsafe extern "C" fn(Object, c_int) -> c_int,
    get_state: unsafe extern "C" fn(Object, *mut c_int, *mut c_int, u64) -> c_int,
    query_position: unsafe extern "C" fn(Object, c_int, *mut i64) -> Gboolean,
    query_duration: unsafe extern "C" fn(Object, c_int, *mut i64) -> Gboolean,
    seek_simple: unsafe extern "C" fn(Object, c_int, c_int, i64) -> Gboolean,
    get_bus: unsafe extern "C" fn(Object) -> Object,
    pop_filtered: unsafe extern "C" fn(Object, c_uint) -> *mut Message,
    parse_error: unsafe extern "C" fn(*mut Message, *mut *mut GError, *mut *mut c_char),
    filename_to_uri: unsafe extern "C" fn(*const c_char, *mut *mut GError) -> *mut c_char,
    static_pad: unsafe extern "C" fn(Object, *const c_char) -> Object,
    current_caps: unsafe extern "C" fn(Object) -> Object,
    caps_structure: unsafe extern "C" fn(Object, c_uint) -> Object,
    structure_int: unsafe extern "C" fn(Object, *const c_char, *mut c_int) -> Gboolean,
    pull_sample: unsafe extern "C" fn(Object, u64) -> Object,
    sample_buffer: unsafe extern "C" fn(Object) -> Object,
    sample_caps: unsafe extern "C" fn(Object) -> Object,
    map: unsafe extern "C" fn(Object, *mut MapInfo, c_int) -> Gboolean,
    unmap: unsafe extern "C" fn(Object, *mut MapInfo),
    unref_mini: unsafe extern "C" fn(*mut c_void),
    unref: unsafe extern "C" fn(Object),
    error_free: unsafe extern "C" fn(*mut GError),
    is_missing: unsafe extern "C" fn(*mut Message) -> Gboolean,
    missing_text: unsafe extern "C" fn(*mut Message) -> *mut c_char,
    free: unsafe extern "C" fn(*mut c_void),
}

fn open(name: &str) -> Result<*mut c_void, String> {
    let c = CString::new(name).expect("no NUL");
    let handle = unsafe { libc::dlopen(c.as_ptr(), libc::RTLD_NOW | libc::RTLD_GLOBAL) };
    if handle.is_null() {
        let raw = unsafe { libc::dlerror() };
        let why = if raw.is_null() { "not found".into() } else { unsafe { CStr::from_ptr(raw) }.to_string_lossy().into_owned() };
        return Err(format!("GStreamer is not installed: {why}"));
    }
    Ok(handle)
}

unsafe fn symbol<F: Copy>(handle: *mut c_void, name: &str) -> Result<F, String> {
    let c = CString::new(name).expect("no NUL");
    let ptr = unsafe { libc::dlsym(handle, c.as_ptr()) };
    if ptr.is_null() {
        return Err(format!("GStreamer lacks {name}"));
    }
    Ok(unsafe { std::mem::transmute_copy(&ptr) })
}

impl Api {
    fn load() -> Result<Api, String> {
        let (core, app) = (open("libgstreamer-1.0.so.0")?, open("libgstapp-1.0.so.0")?);
        let utils = open("libgstpbutils-1.0.so.0")?;
        unsafe {
            let api = Api {
                init_check: symbol(core, "gst_init_check")?,
                make: symbol(core, "gst_element_factory_make")?,
                set: symbol(core, "g_object_set")?,
                get: symbol(core, "g_object_get")?,
                caps_from_string: symbol(core, "gst_caps_from_string")?,
                set_state: symbol(core, "gst_element_set_state")?,
                get_state: symbol(core, "gst_element_get_state")?,
                query_position: symbol(core, "gst_element_query_position")?,
                query_duration: symbol(core, "gst_element_query_duration")?,
                seek_simple: symbol(core, "gst_element_seek_simple")?,
                get_bus: symbol(core, "gst_element_get_bus")?,
                pop_filtered: symbol(core, "gst_bus_pop_filtered")?,
                parse_error: symbol(core, "gst_message_parse_error")?,
                filename_to_uri: symbol(core, "gst_filename_to_uri")?,
                static_pad: symbol(core, "gst_element_get_static_pad")?,
                current_caps: symbol(core, "gst_pad_get_current_caps")?,
                caps_structure: symbol(core, "gst_caps_get_structure")?,
                structure_int: symbol(core, "gst_structure_get_int")?,
                pull_sample: symbol(app, "gst_app_sink_try_pull_sample")?,
                sample_buffer: symbol(core, "gst_sample_get_buffer")?,
                sample_caps: symbol(core, "gst_sample_get_caps")?,
                map: symbol(core, "gst_buffer_map")?,
                unmap: symbol(core, "gst_buffer_unmap")?,
                unref_mini: symbol(core, "gst_mini_object_unref")?,
                unref: symbol(core, "gst_object_unref")?,
                error_free: symbol(core, "g_error_free")?,
                is_missing: symbol(utils, "gst_is_missing_plugin_message")?,
                missing_text: symbol(utils, "gst_missing_plugin_message_get_description")?,
                free: symbol(core, "g_free")?,
            };
            let mut error = std::ptr::null_mut();
            if (api.init_check)(std::ptr::null_mut(), std::ptr::null_mut(), &mut error) == 0 {
                return Err(format!("GStreamer did not start: {}", api.take(error)));
            }
            Ok(api)
        }
    }

    /// A GError's message; the error is freed.
    unsafe fn take(&self, error: *mut GError) -> String {
        if error.is_null() {
            return "GStreamer gave no reason".into();
        }
        let text = unsafe { CStr::from_ptr((*error).message) }.to_string_lossy().into_owned();
        unsafe { (self.error_free)(error) };
        text
    }
}

fn api() -> Result<&'static Api, String> {
    static API: OnceLock<Result<Api, String>> = OnceLock::new();
    API.get_or_init(Api::load).as_ref().map_err(Clone::clone)
}

pub struct Player {
    api: &'static Api,
    playbin: Object,
    sink: Object,
    bus: Object,
    wanted: Cell<bool>,
    ended: Cell<bool>,
    /// The newest sample taken from the sink, or null.
    latest: Cell<Object>,
    failed: std::cell::RefCell<Option<String>>,
}

fn name(text: &str) -> CString {
    CString::new(text).expect("no NUL")
}

impl Player {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let api = api().map_err(anyhow::Error::msg)?;
        let path = CString::new(path.as_os_str().as_bytes())?;
        unsafe {
            let mut error = std::ptr::null_mut();
            let uri = (api.filename_to_uri)(path.as_ptr(), &mut error);
            if uri.is_null() {
                anyhow::bail!(api.take(error));
            }
            let playbin = (api.make)(name("playbin").as_ptr(), std::ptr::null());
            let sink = (api.make)(name("appsink").as_ptr(), std::ptr::null());
            if playbin.is_null() || sink.is_null() {
                (api.free)(uri.cast());
                anyhow::bail!("GStreamer lacks playbin or appsink (gst-plugins-base)");
            }
            let caps = (api.caps_from_string)(name("video/x-raw,format=BGRA").as_ptr());
            // The newest picture only; older ones drop.
            (api.set)(sink, name("caps").as_ptr(), caps, name("max-buffers").as_ptr(), 1 as c_uint, name("drop").as_ptr(), 1 as Gboolean, std::ptr::null::<c_char>());
            (api.unref_mini)(caps);
            (api.set)(playbin, name("uri").as_ptr(), uri, name("video-sink").as_ptr(), sink, std::ptr::null::<c_char>());
            (api.free)(uri.cast());
            let bus = (api.get_bus)(playbin);
            let player = Player { api, playbin, sink, bus, wanted: Cell::new(false), ended: Cell::new(false), latest: Cell::new(std::ptr::null_mut()), failed: Default::default() };
            if (api.set_state)(playbin, STATE_PAUSED) == CHANGE_FAILURE {
                player.read_bus();
                if player.failed.borrow().is_none() {
                    anyhow::bail!("GStreamer would not open the file");
                }
            }
            Ok(player)
        }
    }

    /// Takes errors, missing plugins and end of stream.
    fn read_bus(&self) {
        loop {
            let message = unsafe { (self.api.pop_filtered)(self.bus, MESSAGE_EOS | MESSAGE_ERROR | MESSAGE_ELEMENT) };
            if message.is_null() {
                return;
            }
            let kind = unsafe { (*message).kind };
            if kind == MESSAGE_ELEMENT {
                if unsafe { (self.api.is_missing)(message) } != 0 {
                    let raw = unsafe { (self.api.missing_text)(message) };
                    let what = unsafe { CStr::from_ptr(raw) }.to_string_lossy().into_owned();
                    unsafe { (self.api.free)(raw.cast()) };
                    log::warn!("media: GStreamer lacks a plugin: {what}");
                    self.failed.borrow_mut().get_or_insert(format!("GStreamer lacks a plugin: {what}"));
                }
            } else if kind == MESSAGE_ERROR {
                let (mut error, mut debug) = (std::ptr::null_mut(), std::ptr::null_mut());
                unsafe { (self.api.parse_error)(message, &mut error, &mut debug) };
                let reason = unsafe { self.api.take(error) };
                unsafe { (self.api.free)(debug.cast()) };
                log::warn!("media: GStreamer failed: {reason}");
                self.failed.borrow_mut().get_or_insert(reason);
            } else {
                self.ended.set(true);
            }
            unsafe { (self.api.unref_mini)(message.cast()) };
        }
    }

    fn current(&self) -> c_int {
        let (mut state, mut pending) = (0, 0);
        unsafe { (self.api.get_state)(self.playbin, &mut state, &mut pending, 0) };
        state
    }

    pub fn state(&self) -> State {
        // Bus after state: nothing posted before PAUSED escapes.
        let current = self.current();
        self.read_bus();
        if let Some(reason) = self.failed.borrow().clone() {
            return State::Failed(reason);
        }
        if current < STATE_PAUSED {
            return State::Loading;
        }
        let mut nanos = 0;
        let seconds = match unsafe { (self.api.query_duration)(self.playbin, FORMAT_TIME, &mut nanos) } {
            0 => 0.0,
            _ => nanos as f64 / 1e9,
        };
        State::Ready { seconds, video: self.size() }
    }

    /// The negotiated picture size; none for sound.
    fn size(&self) -> Option<(u32, u32)> {
        let mut videos: c_int = 0;
        unsafe { (self.api.get)(self.playbin, name("n-video").as_ptr(), &mut videos, std::ptr::null::<c_char>()) };
        if videos == 0 {
            return None;
        }
        unsafe {
            let pad = (self.api.static_pad)(self.sink, name("sink").as_ptr());
            let caps = (self.api.current_caps)(pad);
            (self.api.unref)(pad);
            if caps.is_null() {
                return None;
            }
            let size = self.caps_size(caps);
            (self.api.unref_mini)(caps);
            size
        }
    }

    unsafe fn caps_size(&self, caps: Object) -> Option<(u32, u32)> {
        let structure = unsafe { (self.api.caps_structure)(caps, 0) };
        let (mut width, mut height) = (0, 0);
        let read = unsafe {
            (self.api.structure_int)(structure, name("width").as_ptr(), &mut width) != 0
                && (self.api.structure_int)(structure, name("height").as_ptr(), &mut height) != 0
        };
        (read && width > 0 && height > 0).then_some((width as u32, height as u32))
    }

    fn to(&self, state: c_int) {
        if unsafe { (self.api.set_state)(self.playbin, state) } == CHANGE_FAILURE {
            log::warn!("media: GStreamer refused state {state}");
        }
    }

    pub fn play(&self) {
        self.wanted.set(true);
        self.to(STATE_PLAYING);
    }

    pub fn pause(&self) {
        self.wanted.set(false);
        self.to(STATE_PAUSED);
    }

    /// Takes the sink's picture; it holds the stream back.
    fn advance(&self) {
        let sample = unsafe { (self.api.pull_sample)(self.sink, 0) };
        if sample.is_null() {
            return;
        }
        let old = self.latest.replace(sample);
        if !old.is_null() {
            unsafe { (self.api.unref_mini)(old) };
        }
    }

    pub fn playing(&self) -> bool {
        self.advance();
        self.read_bus();
        self.wanted.get() && !self.ended.get() && self.failed.borrow().is_none()
    }

    pub fn position(&self) -> f64 {
        let mut nanos = 0;
        match unsafe { (self.api.query_position)(self.playbin, FORMAT_TIME, &mut nanos) } {
            0 => 0.0,
            _ => nanos as f64 / 1e9,
        }
    }

    pub fn seek(&self, to: f64) {
        self.ended.set(false);
        if unsafe { (self.api.seek_simple)(self.playbin, FORMAT_TIME, SEEK_FLUSH_ACCURATE, (to * 1e9) as i64) } == 0 {
            log::warn!("media: GStreamer refused a seek to {to}");
        }
    }

    pub fn frame(&self) -> Option<Frame> {
        self.advance();
        let sample = self.latest.replace(std::ptr::null_mut());
        if sample.is_null() {
            return None;
        }
        let frame = unsafe { self.copy(sample) };
        unsafe { (self.api.unref_mini)(sample) };
        frame.inspect_err(|error| log::warn!("media: a picture did not copy out: {error}")).ok()
    }

    unsafe fn copy(&self, sample: Object) -> Result<Frame, String> {
        let (width, height) = unsafe { self.caps_size((self.api.sample_caps)(sample)) }.ok_or("a picture without a size")?;
        let buffer = unsafe { (self.api.sample_buffer)(sample) };
        let mut info: MapInfo = unsafe { std::mem::zeroed() };
        if unsafe { (self.api.map)(buffer, &mut info, MAP_READ) } == 0 {
            return Err("the picture would not map".into());
        }
        let want = width as usize * height as usize * 4;
        let copied = (info.size >= want).then(|| unsafe { std::slice::from_raw_parts(info.data, want) }.to_vec());
        unsafe { (self.api.unmap)(buffer, &mut info) };
        let bgra = copied.ok_or_else(|| format!("{} bytes for a {width}x{height} picture", info.size))?;
        Ok(Frame { size: (width, height), bgra })
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        unsafe {
            let sample = self.latest.get();
            if !sample.is_null() {
                (self.api.unref_mini)(sample);
            }
            (self.api.set_state)(self.playbin, STATE_NULL);
            (self.api.unref)(self.bus);
            (self.api.unref)(self.playbin);
        }
    }
}
