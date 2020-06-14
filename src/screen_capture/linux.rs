use std::os::raw::{c_int, c_uint, c_void};
use std::slice::from_raw_parts;

use tracing::warn;

use crate::cerror::CError;
use crate::screen_capture::ScreenCapture;
use crate::x11helper::Capturable;

extern "C" {
    fn start_capture(handle: *const c_void, ctx: *mut c_void, err: *mut CError) -> *mut c_void;
    fn capture_sceen(
        handle: *mut c_void,
        img: *mut CImage,
        capture_cursor: c_int,
        err: *mut CError,
    );
    fn stop_capture(handle: *mut c_void, err: *mut CError);
}

#[repr(C)]
struct CImage {
    data: *const u8,
    width: c_uint,
    height: c_uint,
}

impl CImage {
    pub fn new() -> Self {
        Self {
            data: std::ptr::null(),
            width: 0,
            height: 0,
        }
    }

    pub fn size(&self) -> usize {
        (self.width * self.height * 4) as usize
    }

    pub fn data(&self) -> &[u8] {
        unsafe { from_raw_parts(self.data, self.size()) }
    }
}

pub struct ScreenCaptureX11 {
    handle: *mut c_void,
    img: CImage,
    capture_cursor: bool,
}

impl ScreenCaptureX11 {
    pub fn new(mut capture: Capturable, capture_cursor: bool) -> Result<Self, CError> {
        let mut err = CError::new();
        fltk::app::lock().unwrap();
        let handle = unsafe { start_capture(capture.handle(), std::ptr::null_mut(), &mut err) };
        fltk::app::unlock();
        if err.is_err() {
            return Err(err);
        } else {
            return Ok(Self {
                handle,
                img: CImage::new(),
                capture_cursor,
            });
        }
    }
}

impl Drop for ScreenCaptureX11 {
    fn drop(&mut self) {
        let mut err = CError::new();
        fltk::app::lock().unwrap();
        unsafe {
            stop_capture(self.handle, &mut err);
        }
        fltk::app::unlock();
    }
}

impl ScreenCapture for ScreenCaptureX11 {
    fn capture(&mut self) {
        let mut err = CError::new();
        fltk::app::lock().unwrap();
        unsafe {
            capture_sceen(
                self.handle,
                &mut self.img,
                self.capture_cursor.into(),
                &mut err,
            );
        }
        fltk::app::unlock();
        if err.is_err() {
            warn!("Failed to capture screen: {}", err);
        }
    }

    fn fill_yuv(
        &self,
        y: &mut [u8],
        u: &mut [u8],
        v: &mut [u8],
        y_line_size: usize,
        u_line_size: usize,
        v_line_size: usize,
    ) {
        let data = self.img.data();
        let width = self.img.width as usize;
        let height = self.img.height as usize;

        // Y
        for yy in 0..height - height % 2 {
            for xx in 0..width - width % 2 {
                let i = 4 * (width * yy + xx);
                let b = data[i] as i32;
                let g = data[i + 1] as i32;
                let r = data[i + 2] as i32;
                y[y_line_size * yy + xx] = (((66 * r + 129 * g + 25 * b + 128) >> 8) + 16) as u8;
            }
        }

        let y_len = 4 * width;
        // Cb and Cr
        for yy in 0..(height / 2) {
            for xx in 0..(width / 2) {
                let i = 8 * (yy * width + xx);
                let mut b = data[i] as i32 + data[i + 4] as i32;
                let mut g = data[i + 1] as i32 + data[i + 1 + 4] as i32;
                let mut r = data[i + 2] as i32 + data[i + 2 + 4] as i32;
                b += data[i + y_len] as i32 + data[i + 4 + y_len] as i32;
                g += data[i + 1 + y_len] as i32 + data[i + 1 + 4 + y_len] as i32;
                r += data[i + 2 + y_len] as i32 + data[i + 2 + 4 + y_len] as i32;
                r >>= 2;
                g >>= 2;
                b >>= 2;
                u[yy * u_line_size + xx] = (((128 + 112 * b - 38 * r - 74 * g) >> 8) + 128) as u8;
                v[yy * v_line_size + xx] = (((128 + 112 * r - 94 * g - 18 * b) >> 8) + 128) as u8;
            }
        }
    }

    fn size(&self) -> (usize, usize) {
        (self.img.width as usize, self.img.height as usize)
    }
}
