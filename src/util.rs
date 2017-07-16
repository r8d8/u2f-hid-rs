extern crate libc;

use std::error::Error;
use std::{io, mem, slice};
use std::sync::{Arc, Mutex};
use boxfnonce::SendBoxFnOnce;

macro_rules! try_or {
    ($val:expr, $or:expr) => {
        match $val {
            Ok(v) => { v }
            Err(e) => { return $or(e); }
        }
    }
}

pub trait Signed {
    fn is_negative(&self) -> bool;
}

impl Signed for i32 {
    fn is_negative(&self) -> bool {
        *self < (0 as i32)
    }
}

impl Signed for usize {
    fn is_negative(&self) -> bool {
        (*self as isize) < (0 as isize)
    }
}

#[cfg(any(target_os = "linux"))]
pub fn from_unix_result<T: Signed>(rv: T) -> io::Result<T> {
    if rv.is_negative() {
        let errno = unsafe { *libc::__errno_location() };
        Err(io::Error::from_raw_os_error(errno))
    } else {
        Ok(rv)
    }
}

pub fn to_u8_array<T>(non_ptr: &T) -> &[u8] {
    unsafe { slice::from_raw_parts(non_ptr as *const T as *const u8, mem::size_of::<T>()) }
}

pub fn from_u8_array<T>(arr: &[u8]) -> &T {
    unsafe { &*(arr.as_ptr() as *const T) }
}

pub fn set_data(data: &mut [u8], itr: &mut slice::Iter<u8>, max: usize) {
    let take_amount;
    let count = itr.size_hint().0;
    if max < count {
        take_amount = max;
    } else {
        take_amount = count;
    }
    // TODO There is a better way to do this :|
    for i in 0..take_amount {
        data[i] = *itr.next().unwrap();
    }
}

pub fn io_err(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::Other, msg)
}

pub fn to_io_err<T: Error>(err: T) -> io::Error {
    io_err(err.description())
}

type Callback = SendBoxFnOnce<(io::Result<Vec<u8>>,)>;

pub struct OnceCallback {
    callback: Arc<Mutex<Option<Callback>>>,
}

impl OnceCallback {
    pub fn new<F>(cb: F) -> Self
    where
        F: FnOnce(io::Result<Vec<u8>>),
        F: Send + 'static,
    {
        let cb = Some(SendBoxFnOnce::from(cb));
        Self { callback: Arc::new(Mutex::new(cb)) }
    }

    pub fn call(&self, rv: io::Result<Vec<u8>>) {
        if let Ok(mut cb) = self.callback.lock() {
            if let Some(cb) = cb.take() {
                cb.call(rv);
            }
        }
    }
}

impl Clone for OnceCallback {
    fn clone(&self) -> Self {
        Self { callback: self.callback.clone() }
    }
}

pub fn to_hex_string(bytes: &Vec<u8>) -> String {
    let strs: Vec<String> = bytes.iter()
        .map(|b| format!("{:02x}", b))
        .collect();
    strs.join("")
}