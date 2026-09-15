use soapysdr_sys::*;
use std::ptr;
use std::slice;

use std::ffi::CStr;
use std::os::raw::c_char;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[non_exhaustive]
pub enum ArgType {
    Bool,
    Float,
    Int,
    String,
}

impl From<SoapySDRArgInfoType> for ArgType {
    /// SoapySDR up to and including 0.8.1 ships an `ArgInfo` default constructor that leaves
    /// `type` uninitialized, so a driver that pushes a default-constructed `ArgInfo` hands us
    /// whatever was on its stack. Fall back to the `STRING` the later constructor settled on
    /// rather than aborting the caller.
    #[allow(non_upper_case_globals)]
    fn from(arg_info_type: SoapySDRArgInfoType) -> Self {
        match arg_info_type {
            soapysdr_sys::SOAPY_SDR_ARG_INFO_BOOL => ArgType::Bool,
            soapysdr_sys::SOAPY_SDR_ARG_INFO_FLOAT => ArgType::Float,
            soapysdr_sys::SOAPY_SDR_ARG_INFO_INT => ArgType::Int,
            _ => ArgType::String,
        }
    }
}

/// Metadata about supported arguments.
#[derive(Debug)]
pub struct ArgInfo {
    /// The key used to identify the argument
    pub key: String,

    /// The default value of the argument when not specified
    pub value: String,

    /// The displayable name of the argument
    pub name: Option<String>,

    ///  A brief description about the argument
    pub description: Option<String>,

    /// The units of the argument: dB, Hz, etc
    pub units: Option<String>,

    /// The data type of the argument
    pub data_type: ArgType,

    /// The allowed numeric range, when the driver declares one.
    pub range: Option<SoapySDRRange>,

    /// A discrete list of possible values.
    ///
    /// When specified, the argument should be restricted to this options set.
    pub options: Vec<(String, Option<String>)>,
}

unsafe fn required_string(s: *mut c_char) -> String {
    unsafe { optional_string(s).unwrap_or_default() }
}

unsafe fn optional_string(s: *mut c_char) -> Option<String> {
    unsafe {
        if !s.is_null() {
            Some(CStr::from_ptr(s).to_string_lossy().into())
        } else {
            None
        }
    }
}

unsafe fn options_from_c(c: &SoapySDRArgInfo) -> Vec<(String, Option<String>)> {
    unsafe {
        if c.numOptions == 0 || c.options.is_null() {
            return Vec::new();
        }
        let option_vals = slice::from_raw_parts(c.options, c.numOptions);
        let option_names = if c.optionNames.is_null() {
            &[][..]
        } else {
            slice::from_raw_parts(c.optionNames, c.numOptions)
        };
        option_vals
            .iter()
            .enumerate()
            .map(|(index, &value)| {
                let label = option_names.get(index).copied().unwrap_or(ptr::null_mut());
                (required_string(value), optional_string(label))
            })
            .collect()
    }
}

pub unsafe fn arg_info_from_c(c: &SoapySDRArgInfo) -> ArgInfo {
    unsafe {
        ArgInfo {
            key: required_string(c.key),
            value: required_string(c.value),
            name: optional_string(c.name),
            description: optional_string(c.description),
            units: optional_string(c.units),
            data_type: c.type_.into(),
            range: ((c.range.minimum != 0.0) || (c.range.maximum != 0.0) || (c.range.step != 0.0))
                .then_some(c.range),
            options: options_from_c(c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    fn owned(text: &str) -> *mut c_char {
        CString::new(text)
            .expect("test string has no null byte")
            .into_raw()
    }

    unsafe fn release(s: *mut c_char) {
        if !s.is_null() {
            drop(unsafe { CString::from_raw(s) });
        }
    }

    #[test]
    fn known_arg_types_map_across() {
        assert_eq!(ArgType::from(SOAPY_SDR_ARG_INFO_BOOL), ArgType::Bool);
        assert_eq!(ArgType::from(SOAPY_SDR_ARG_INFO_INT), ArgType::Int);
        assert_eq!(ArgType::from(SOAPY_SDR_ARG_INFO_FLOAT), ArgType::Float);
        assert_eq!(ArgType::from(SOAPY_SDR_ARG_INFO_STRING), ArgType::String);
    }

    #[test]
    fn unknown_arg_type_reads_as_string() {
        assert_eq!(ArgType::from(4 as SoapySDRArgInfoType), ArgType::String);
        assert_eq!(ArgType::from(SoapySDRArgInfoType::MAX), ArgType::String);
    }

    #[test]
    fn default_constructed_arg_info_converts_without_panicking() {
        let mut c: SoapySDRArgInfo = unsafe { std::mem::zeroed() };
        c.key = owned("");
        c.value = owned("");
        c.type_ = 0xdead_beef;

        let info = unsafe { arg_info_from_c(&c) };

        assert!(info.key.is_empty());
        assert_eq!(info.data_type, ArgType::String);
        assert_eq!(info.name, None);
        assert!(info.range.is_none());
        assert!(info.options.is_empty());

        unsafe {
            release(c.key);
            release(c.value);
        }
    }

    #[test]
    fn options_survive_a_missing_label_list() {
        let mut values = vec![owned("auto"), owned("meta")];
        let mut c: SoapySDRArgInfo = unsafe { std::mem::zeroed() };
        c.key = owned("meta");
        c.value = owned("auto");
        c.type_ = SOAPY_SDR_ARG_INFO_STRING;
        c.numOptions = values.len();
        c.options = values.as_mut_ptr();

        let info = unsafe { arg_info_from_c(&c) };

        assert_eq!(
            info.options,
            vec![("auto".to_string(), None), ("meta".to_string(), None)]
        );

        unsafe {
            release(c.key);
            release(c.value);
            for value in values.drain(..) {
                release(value);
            }
        }
    }

    #[test]
    fn a_null_option_list_is_no_options() {
        let mut c: SoapySDRArgInfo = unsafe { std::mem::zeroed() };
        c.key = owned("meta");
        c.value = owned("auto");
        c.type_ = SOAPY_SDR_ARG_INFO_STRING;
        c.numOptions = 3;

        let info = unsafe { arg_info_from_c(&c) };

        assert!(info.options.is_empty());

        unsafe {
            release(c.key);
            release(c.value);
        }
    }
}
