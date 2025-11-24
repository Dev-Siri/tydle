use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

use crate::{Extract, Tydle, TydleOptions, VideoId};
use anyhow::Result;

#[unsafe(no_mangle)]
pub extern "C" fn tydle_new() -> *mut Tydle {
    let ty = Tydle::new(TydleOptions::default()).unwrap();
    Box::into_raw(Box::new(ty))
}

#[unsafe(no_mangle)]
pub extern "C" fn tydle_free(ptr: *mut Tydle) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(ptr));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tydle_get_manifest_json(
    tydle_ptr: *mut Tydle,
    video_id: *const c_char,
) -> *mut c_char {
    if tydle_ptr.is_null() {
        return ptr::null_mut();
    }

    let c_str = unsafe { CStr::from_ptr(video_id) };
    let id_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let vid = match VideoId::new(id_str) {
        Ok(v) => v,
        Err(_) => return ptr::null_mut(),
    };

    let tydle = unsafe { &*tydle_ptr };
    let rt = tokio::runtime::Runtime::new().unwrap();

    let result: Result<_> = rt.block_on(async move {
        let manifest = tydle.get_manifest(&vid).await?;
        Ok(serde_json::to_string(&manifest)?)
    });

    match result {
        Ok(json) => CString::new(json).unwrap().into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tydle_get_streams_json(
    tydle_ptr: *mut Tydle,
    video_id: *const c_char,
) -> *mut c_char {
    if tydle_ptr.is_null() {
        return ptr::null_mut();
    }

    let c_str = unsafe { CStr::from_ptr(video_id) };
    let id_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let vid = match VideoId::new(id_str) {
        Ok(v) => v,
        Err(_) => return ptr::null_mut(),
    };

    let tydle = unsafe { &*tydle_ptr };
    let rt = tokio::runtime::Runtime::new().unwrap();

    let result: Result<_> = rt.block_on(async move {
        let streams = tydle.get_streams(&vid).await?;
        Ok(serde_json::to_string(&streams)?)
    });

    match result {
        Ok(json) => CString::new(json).unwrap().into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tydle_get_video_info_json(
    tydle_ptr: *mut Tydle,
    video_id: *const c_char,
) -> *mut c_char {
    if tydle_ptr.is_null() {
        return ptr::null_mut();
    }

    let c_str = unsafe { CStr::from_ptr(video_id) };
    let id_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let vid = match VideoId::new(id_str) {
        Ok(v) => v,
        Err(_) => return ptr::null_mut(),
    };

    let tydle = unsafe { &*tydle_ptr };
    let rt = tokio::runtime::Runtime::new().unwrap();

    let result: Result<_> = rt.block_on(async move {
        let info = tydle.get_video_info(&vid).await?;
        Ok(serde_json::to_string(&info)?)
    });

    match result {
        Ok(json) => CString::new(json).unwrap().into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tydle_streams_from_manifest_json(
    tydle_ptr: *mut Tydle,
    manifest_json: *const c_char,
) -> *mut c_char {
    if tydle_ptr.is_null() {
        return ptr::null_mut();
    }

    let c_str = unsafe { CStr::from_ptr(manifest_json) };
    let manifest_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let manifest: crate::YtManifest = match serde_json::from_str(manifest_str) {
        Ok(m) => m,
        Err(_) => return ptr::null_mut(),
    };

    let tydle = unsafe { &*tydle_ptr };
    let rt = tokio::runtime::Runtime::new().unwrap();

    let result: Result<_> = rt.block_on(async move {
        let streams = tydle.get_streams_from_manifest(&manifest).await?;
        Ok(serde_json::to_string(&streams)?)
    });

    match result {
        Ok(json) => CString::new(json).unwrap().into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tydle_video_info_from_manifest_json(
    tydle_ptr: *mut Tydle,
    manifest_json: *const c_char,
) -> *mut c_char {
    if tydle_ptr.is_null() {
        return ptr::null_mut();
    }

    let c_str = unsafe { CStr::from_ptr(manifest_json) };
    let manifest_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let manifest: crate::YtManifest = match serde_json::from_str(manifest_str) {
        Ok(m) => m,
        Err(_) => return ptr::null_mut(),
    };

    let tydle = unsafe { &*tydle_ptr };
    let rt = tokio::runtime::Runtime::new().unwrap();

    let result: Result<_> = rt.block_on(async move {
        let info = tydle.get_video_info_from_manifest(&manifest).await?;
        Ok(serde_json::to_string(&info)?)
    });

    match result {
        Ok(json) => CString::new(json).unwrap().into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tydle_string_free(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    unsafe {
        drop(CString::from_raw(s));
    }
}
