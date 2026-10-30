//! The worker stays out of the Dock (macOS).

use std::ffi::{c_char, c_void};
use std::io;

#[link(name = "Foundation", kind = "framework")]
#[link(name = "objc")]
unsafe extern "C" {
    fn objc_getClass(name: *const c_char) -> *mut c_void;
    fn sel_registerName(name: *const c_char) -> *mut c_void;
    fn objc_msgSend();
}

/// NSApplicationActivationPolicyAccessory: no Dock icon, no menu bar.
const ACCESSORY: isize = 1;
/// NSApplicationActivationPolicyProhibited: an unbundled worker starts so.
const PROHIBITED: isize = 2;

type Id = *mut c_void;

fn send() -> unsafe extern "C" fn() {
    objc_msgSend as unsafe extern "C" fn()
}

fn class(name: &std::ffi::CStr) -> io::Result<Id> {
    let class = unsafe { objc_getClass(name.as_ptr()) };
    if class.is_null() {
        return Err(io::Error::other(format!("no {} class", name.to_string_lossy())));
    }
    Ok(class)
}

fn get(object: Id, selector: &std::ffi::CStr) -> io::Result<Id> {
    type F = unsafe extern "C" fn(Id, Id) -> Id;
    let f: F = unsafe { std::mem::transmute(send()) };
    let result = unsafe { f(object, sel_registerName(selector.as_ptr())) };
    if result.is_null() {
        return Err(io::Error::other(format!("{} gave nothing", selector.to_string_lossy())));
    }
    Ok(result)
}

/// Marks the process a UI element before registration.
pub fn hide() -> io::Result<()> {
    // The kit reads the bundle's LSUIElement when it registers.
    let info = get(get(class(c"NSBundle")?, c"mainBundle")?, c"infoDictionary")?;
    type Text = unsafe extern "C" fn(Id, Id, *const c_char) -> Id;
    type Flag = unsafe extern "C" fn(Id, Id, i8) -> Id;
    type Set = unsafe extern "C" fn(Id, Id, Id, Id);
    let text: Text = unsafe { std::mem::transmute(send()) };
    let flag: Flag = unsafe { std::mem::transmute(send()) };
    let set: Set = unsafe { std::mem::transmute(send()) };
    unsafe {
        let key = text(class(c"NSString")?, sel_registerName(c"stringWithUTF8String:".as_ptr()), c"LSUIElement".as_ptr());
        let yes = flag(class(c"NSNumber")?, sel_registerName(c"numberWithBool:".as_ptr()), 1);
        set(info, sel_registerName(c"setObject:forKey:".as_ptr()), yes, key);
    }
    Ok(())
}

/// Fails unless the kernel's application came up an accessory.
pub fn check() -> io::Result<()> {
    type Policy = unsafe extern "C" fn(Id, Id) -> isize;
    type SetPolicy = unsafe extern "C" fn(Id, Id, isize) -> i8;
    let policy: Policy = unsafe { std::mem::transmute(send()) };
    let set: SetPolicy = unsafe { std::mem::transmute(send()) };
    let app = get(class(c"NSApplication")?, c"sharedApplication")?;
    let read = || unsafe { policy(app, sel_registerName(c"activationPolicy".as_ptr())) };
    let mut found = read();
    if found == PROHIBITED {
        // Outside a bundle nothing registers; accessory lets it draw.
        let done = unsafe { set(app, sel_registerName(c"setActivationPolicy:".as_ptr()), ACCESSORY) };
        // A granted change lands a moment later.
        let asked = std::time::Instant::now();
        found = read();
        while done != 0 && found == PROHIBITED && asked.elapsed() < std::time::Duration::from_secs(2) {
            std::thread::sleep(std::time::Duration::from_millis(5));
            found = read();
        }
        log::info!("lok: the unbundled worker asked for accessory ({done}), in {:?}", asked.elapsed());
    }
    if found != ACCESSORY {
        return Err(io::Error::other(format!("the worker's activation policy is {found}, not accessory")));
    }
    log::info!("lok: the worker is an accessory app");
    Ok(())
}
