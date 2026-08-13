//! [SoapySDR](https://github.com/pothosware/SoapySDR/wiki) provides a hardware abstraction layer
//! for transmitting and receiving with many software defined radio devices.
//!
//!

mod args;
pub use args::{Args, ArgsIterator};

mod arginfo;
pub use arginfo::{ArgInfo, ArgType};

mod device;
pub use device::{Device, Direction, Error, ErrorCode, Range, RxStream, TxStream, enumerate};

mod format;
pub use format::{Format, StreamSample};

fn static_string(ptr: *const std::os::raw::c_char) -> String {
    unsafe { std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned() }
}

fn owned_string_list(
    function: unsafe extern "C" fn(*mut usize) -> *mut *mut std::os::raw::c_char,
) -> Vec<String> {
    unsafe {
        let mut len = 0usize;
        let mut ptr = function(&mut len);
        let values = std::slice::from_raw_parts(ptr, len)
            .iter()
            .map(|item| {
                std::ffi::CStr::from_ptr(*item)
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        soapysdr_sys::SoapySDRStrings_clear(&mut ptr, len);
        values
    }
}

/// The loaded SoapySDR core library version and build information.
pub fn library_version() -> String {
    unsafe { static_string(soapysdr_sys::SoapySDR_getLibVersion()) }
}

/// The module directories the loaded SoapySDR core will search.
pub fn module_search_paths() -> Vec<String> {
    owned_string_list(soapysdr_sys::SoapySDR_listSearchPaths)
}

/// The loadable modules found in the configured search paths.
pub fn list_modules() -> Vec<String> {
    owned_string_list(soapysdr_sys::SoapySDR_listModules)
}

/// Configures SoapySDR to log to the Rust `log` facility.
///
/// With `env_logger`, use e.g `RUST_LOG=soapysdr=info` to control the log level.
#[cfg(feature = "log")]
pub fn configure_logging() {
    use log::Level;
    use log::log;
    use soapysdr_sys::*;
    use std::ffi::CStr;
    use std::os::raw::c_char;

    extern "C" fn soapy_log(level: SoapySDRLogLevel, message: *const c_char) {
        #![allow(non_upper_case_globals)]
        let level = match level {
            soapysdr_sys::SOAPY_SDR_FATAL => Level::Error,
            soapysdr_sys::SOAPY_SDR_CRITICAL => Level::Error,
            soapysdr_sys::SOAPY_SDR_ERROR => Level::Error,
            soapysdr_sys::SOAPY_SDR_WARNING => Level::Warn,
            soapysdr_sys::SOAPY_SDR_NOTICE => Level::Info,
            soapysdr_sys::SOAPY_SDR_INFO => Level::Info,
            soapysdr_sys::SOAPY_SDR_DEBUG => Level::Debug,
            soapysdr_sys::SOAPY_SDR_TRACE => Level::Trace,
            soapysdr_sys::SOAPY_SDR_SSI => Level::Info, // Streaming status indicators such as "U" (underflow) and "O" (overflow).
            _ => Level::Error,
        };

        let msg = unsafe { CStr::from_ptr(message) };
        log!(
            level,
            "{}",
            msg.to_string_lossy().trim_start_matches(&['\r', '\n'][..])
        );
    }

    unsafe {
        SoapySDR_registerLogHandler(Some(soapy_log));
    }
}
