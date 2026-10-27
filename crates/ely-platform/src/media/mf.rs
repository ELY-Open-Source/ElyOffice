use std::{cell::Cell, path::Path};

use windows::{
    Win32::{
        Foundation::{RECT, RPC_E_CHANGED_MODE, S_OK},
        Graphics::{
            Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
            Imaging::{CLSID_WICImagingFactory, GUID_WICPixelFormat32bppBGRA, IWICBitmap, IWICImagingFactory, WICBitmapCacheOnLoad, WICBitmapLockRead, WICRect},
        },
        Media::MediaFoundation::{
            CLSID_MFMediaEngineClassFactory, IMFAttributes, IMFMediaEngine, IMFMediaEngineClassFactory, IMFMediaEngineEx, IMFMediaEngineNotify,
            IMFMediaEngineNotify_Impl, MF_MEDIA_ENGINE_CALLBACK, MF_MEDIA_ENGINE_READY_HAVE_METADATA, MF_MEDIA_ENGINE_VIDEO_OUTPUT_FORMAT,
            MF_ACCESSMODE_READ, MF_FILEFLAGS_NONE, MF_OPENMODE_FAIL_IF_NOT_EXIST, MF_VERSION, MFCreateAttributes, MFCreateFile, MFSTARTUP_FULL, MFStartup,
        },
        System::Com::{CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx},
    },
    core::{BSTR, Interface, implement},
};

use super::{Frame, State};

/// The engine's events; state is read when asked.
#[implement(IMFMediaEngineNotify)]
struct Quiet;

impl IMFMediaEngineNotify_Impl for Quiet_Impl {
    fn EventNotify(&self, _event: u32, _param1: usize, _param2: u32) -> windows::core::Result<()> {
        Ok(())
    }
}

pub struct Player {
    engine: IMFMediaEngine,
    imaging: IWICImagingFactory,
    /// The newest picture's time stamp, and the one copied out.
    latest: Cell<Option<i64>>,
    shown: Cell<Option<i64>>,
    canvas: std::cell::RefCell<Option<(IWICBitmap, u32, u32)>>,
}

/// COM on this thread, in whichever mode it already has.
fn com() -> anyhow::Result<()> {
    let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    // A thread already in single-threaded COM keeps it.
    if hr.is_ok() || hr == RPC_E_CHANGED_MODE {
        return Ok(());
    }
    Err(windows::core::Error::from(hr).into())
}

impl Player {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        com()?;
        unsafe { MFStartup(MF_VERSION, MFSTARTUP_FULL) }?;
        let factory: IMFMediaEngineClassFactory = unsafe { CoCreateInstance(&CLSID_MFMediaEngineClassFactory, None, CLSCTX_INPROC_SERVER) }?;
        let mut attributes: Option<IMFAttributes> = None;
        unsafe { MFCreateAttributes(&mut attributes, 2) }?;
        let attributes = attributes.ok_or_else(|| anyhow::anyhow!("Media Foundation made no attributes"))?;
        let notify: IMFMediaEngineNotify = Quiet.into();
        unsafe {
            attributes.SetUnknown(&MF_MEDIA_ENGINE_CALLBACK, &notify)?;
            // Frames come out on request, not to a window.
            attributes.SetUINT32(&MF_MEDIA_ENGINE_VIDEO_OUTPUT_FORMAT, DXGI_FORMAT_B8G8R8A8_UNORM.0 as u32)?;
        }
        let engine = unsafe { factory.CreateInstance(0, &attributes) }?;
        let path = dunce::canonicalize(path)?;
        let name = windows::core::HSTRING::from(path.as_os_str());
        // Bytes need no URL; the name hints the format.
        let stream = unsafe { MFCreateFile(MF_ACCESSMODE_READ, MF_OPENMODE_FAIL_IF_NOT_EXIST, MF_FILEFLAGS_NONE, &name) }?;
        let engine_ex: IMFMediaEngineEx = engine.cast()?;
        unsafe { engine_ex.SetSourceFromByteStream(&stream, &BSTR::from(path.to_string_lossy().as_ref())) }?;
        let imaging = unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER) }?;
        Ok(Player { engine, imaging, latest: Cell::new(None), shown: Cell::new(None), canvas: Default::default() })
    }

    pub fn state(&self) -> State {
        match unsafe { self.engine.GetError() } {
            Ok(error) => {
                let reason = unsafe { error.GetExtendedErrorCode() }.err().map(|error| error.message());
                let code = unsafe { error.GetErrorCode() };
                return State::Failed(reason.filter(|reason| !reason.is_empty()).unwrap_or_else(|| format!("Media Foundation error {code}")));
            }
            // No error object comes back as an empty pointer.
            Err(error) if error.code().is_ok() => {}
            Err(error) => return State::Failed(error.message()),
        }
        if i32::from(unsafe { self.engine.GetReadyState() }) < MF_MEDIA_ENGINE_READY_HAVE_METADATA.0 {
            return State::Loading;
        }
        let duration = unsafe { self.engine.GetDuration() };
        State::Ready { seconds: if duration.is_finite() { duration } else { 0.0 }, video: self.size() }
    }

    fn size(&self) -> Option<(u32, u32)> {
        if !unsafe { self.engine.HasVideo() }.as_bool() {
            return None;
        }
        let (mut width, mut height) = (0, 0);
        unsafe { self.engine.GetNativeVideoSize(Some(&mut width), Some(&mut height)) }.ok()?;
        (width > 0 && height > 0).then_some((width, height))
    }

    pub fn play(&self) {
        if let Err(error) = unsafe { self.engine.Play() } {
            log::warn!("media: play failed: {error}");
        }
    }

    pub fn pause(&self) {
        if let Err(error) = unsafe { self.engine.Pause() } {
            log::warn!("media: pause failed: {error}");
        }
    }

    /// Lets the engine present; it stalls when nothing asks.
    fn advance(&self) {
        if !unsafe { self.engine.HasVideo() }.as_bool() {
            return;
        }
        let mut stamp = 0;
        // S_FALSE, no new picture, must not read as one.
        let hr = unsafe { (Interface::vtable(&self.engine).OnVideoStreamTick)(self.engine.as_raw(), &mut stamp) };
        if hr == S_OK {
            self.latest.set(Some(stamp));
        } else if hr.is_err() {
            log::warn!("media: no picture time: {}", windows::core::Error::from(hr));
        }
    }

    pub fn playing(&self) -> bool {
        self.advance();
        !unsafe { self.engine.IsPaused() }.as_bool() && !unsafe { self.engine.IsEnded() }.as_bool()
    }

    pub fn position(&self) -> f64 {
        unsafe { self.engine.GetCurrentTime() }
    }

    pub fn seek(&self, to: f64) {
        if let Err(error) = unsafe { self.engine.SetCurrentTime(to) } {
            log::warn!("media: seek failed: {error}");
        }
    }

    pub fn frame(&self) -> Option<Frame> {
        self.advance();
        let (width, height) = self.size()?;
        let stamp = self.latest.get()?;
        if self.shown.get() == Some(stamp) {
            return None;
        }
        match self.copy((width, height)) {
            Ok(frame) => {
                self.shown.set(Some(stamp));
                Some(frame)
            }
            Err(error) => {
                log::warn!("media: a picture did not copy out: {error}");
                None
            }
        }
    }

    /// The engine's current picture as BGRA rows.
    fn copy(&self, (width, height): (u32, u32)) -> windows::core::Result<Frame> {
        let mut canvas = self.canvas.borrow_mut();
        if !canvas.as_ref().is_some_and(|(_, w, h)| (*w, *h) == (width, height)) {
            let bitmap = unsafe { self.imaging.CreateBitmap(width, height, &GUID_WICPixelFormat32bppBGRA, WICBitmapCacheOnLoad) }?;
            *canvas = Some((bitmap, width, height));
        }
        let (bitmap, ..) = canvas.as_ref().expect("made above");
        let whole = RECT { left: 0, top: 0, right: width as i32, bottom: height as i32 };
        unsafe { self.engine.TransferVideoFrame(bitmap, None, &whole, None) }?;
        let lock = unsafe { bitmap.Lock(&WICRect { X: 0, Y: 0, Width: width as i32, Height: height as i32 }, WICBitmapLockRead.0 as u32) }?;
        let stride = unsafe { lock.GetStride() }? as usize;
        let (mut length, mut data) = (0, std::ptr::null_mut());
        unsafe { lock.GetDataPointer(&mut length, &mut data) }?;
        let rows = unsafe { std::slice::from_raw_parts(data, length as usize) };
        let row = width as usize * 4;
        let bgra = (0..height as usize).flat_map(|y| &rows[y * stride..y * stride + row]).copied().collect();
        Ok(Frame { size: (width, height), bgra })
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        if let Err(error) = unsafe { self.engine.Shutdown() } {
            log::warn!("media: the engine did not shut down: {error}");
        }
    }
}
