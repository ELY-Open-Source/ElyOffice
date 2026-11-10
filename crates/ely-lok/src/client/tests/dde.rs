//! A LOK worker serves no DDE (R15.70).

use std::ptr::{null, null_mut};

use windows_sys::Win32::System::DataExchange::{
    APPCMD_CLIENTONLY, CONVINFO, CP_WINUNICODE, DdeConnectList, DdeCreateStringHandleW, DdeDisconnectList, DdeFreeStringHandle, DdeInitializeW,
    DdeQueryConvInfo, DdeQueryNextServer, DdeUninitialize, HCONV, HDDEDATA, HSZ, QID_SYNC,
};
use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;

use super::*;

/// A client takes no callbacks worth answering.
unsafe extern "system" fn quiet(_: u32, _: u32, _: HCONV, _: HSZ, _: HSZ, _: HDDEDATA, _: usize, _: usize) -> HDDEDATA {
    null_mut()
}

/// Every process that answers `service|topic` over DDE.
fn answerers(inst: u32, service: &str, topic: &str) -> Vec<u32> {
    let named = |name: &str| {
        let wide: Vec<u16> = name.encode_utf16().chain([0]).collect();
        let handle = unsafe { DdeCreateStringHandleW(inst, wide.as_ptr(), CP_WINUNICODE) };
        assert!(!handle.is_null(), "no DDE string for {name}");
        handle
    };
    let (service, topic) = (named(service), named(topic));
    let list = unsafe { DdeConnectList(inst, service, topic, null_mut(), null()) };
    let mut pids = Vec::new();
    let mut conv = unsafe { DdeQueryNextServer(list, null_mut()) };
    while !conv.is_null() {
        let mut info = CONVINFO { cb: size_of::<CONVINFO>() as u32, ..unsafe { std::mem::zeroed() } };
        assert_ne!(unsafe { DdeQueryConvInfo(conv, QID_SYNC, &mut info) }, 0, "a conversation tells nothing");
        let mut pid = 0;
        unsafe { GetWindowThreadProcessId(info.hwndPartner, &mut pid) };
        pids.push(pid);
        conv = unsafe { DdeQueryNextServer(list, conv) };
    }
    unsafe {
        DdeDisconnectList(list);
        DdeFreeStringHandle(inst, service);
        DdeFreeStringHandle(inst, topic);
    }
    pids
}

#[test]
fn a_worker_answers_no_dde_service_name() {
    let Some(install) = install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let source = dir.path().join("note.fodt");
    write(&source, FODT);
    // A loaded document: the kernel is up and pumping messages.
    let _doc = worker.load(&crate::file_url(&source)).expect("the document loads");
    let pid = worker.child.id();
    let mut inst = 0;
    assert_eq!(unsafe { DdeInitializeW(&mut inst, Some(quiet), APPCMD_CLIENTONLY, 0) }, 0, "DDEML does not start");
    // Other programs may answer; this worker must not.
    for (service, topic) in [("soffice", "System"), ("elynosuchapp", "nofile")] {
        let pids = answerers(inst, service, topic);
        assert!(!pids.contains(&pid), "worker {pid} answers {service}|{topic} among {pids:?}");
    }
    unsafe { DdeUninitialize(inst) };
}
