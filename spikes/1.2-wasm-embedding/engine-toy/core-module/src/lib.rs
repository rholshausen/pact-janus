//! Core-WASM build of the toy engine: the same byte-pipe, hand-shimmed over a
//! C ABI + linear memory for hosts without component-model support.
//!
//! Convention (every core host re-implements this by hand — that is the point
//! being measured):
//!   1. host calls `alloc(req_len)`, copies the request frame into guest memory
//!   2. host calls `call(req_ptr, req_len)` -> packed u64: (resp_ptr << 32) | resp_len
//!   3. host copies the response frame out of guest memory
//!   4. host calls `dealloc(resp_ptr, resp_len)` and `dealloc(req_ptr, req_len)`

use std::alloc::Layout;

#[no_mangle]
pub extern "C" fn alloc(len: u32) -> *mut u8 {
    let layout = Layout::from_size_align(len as usize, 1).expect("valid layout");
    unsafe { std::alloc::alloc(layout) }
}

/// # Safety
/// `ptr` must be a pointer previously returned by `alloc(len)` or by `call`.
#[no_mangle]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: u32) {
    let layout = Layout::from_size_align(len as usize, 1).expect("valid layout");
    unsafe { std::alloc::dealloc(ptr, layout) }
}

/// # Safety
/// `ptr`/`len` must describe a readable region of guest memory.
#[no_mangle]
pub unsafe extern "C" fn call(ptr: *const u8, len: u32) -> u64 {
    let request = unsafe { std::slice::from_raw_parts(ptr, len as usize) };
    let response = toy_logic::handle_frame(request);
    let resp_len = response.len() as u32;
    let mut boxed = response.into_boxed_slice();
    let resp_ptr = boxed.as_mut_ptr();
    std::mem::forget(boxed);
    ((resp_ptr as u64) << 32) | resp_len as u64
}
