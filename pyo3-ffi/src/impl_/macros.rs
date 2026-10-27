// On x86 Windows, `raw-dylib` with `import_name_type = "undecorated"` removes the
// leading cdecl underscore from function names. This is expected behavior for
// `import_name_type = "undecorated"` (not a rustc bug): it strips the cdecl `_`
// prefix, which collides with symbols whose real names start with `_Py`.
// See https://doc.rust-lang.org/reference/items/external-blocks.html#the-import_name_type-key
//
// That matches ordinary `Py_*` exports, but it breaks CPython's internal `_Py*`
// function exports whose real DLL names already start with an underscore. For
// those functions, ask rustc for one extra underscore so that x86 undecoration
// lands back on CPython's export.
//
// Variables are intentionally excluded here: `import_name_type` does not affect
// variable imports, so `_Py_*` statics continue to work without any rewriting.
#[allow(unused_macros, reason = "used indirectly by extern_libpython_item!")]
macro_rules! extern_libpython_cpython_private_fn {
    ($(#[$attrs:meta])* $vis:vis $name:ident($($args:tt)*) $(-> $ret:ty)?) => {
        #[cfg_attr(
            all(windows, pyo3_use_raw_dylib, target_arch = "x86"),
            link_name = concat!("_", stringify!($name))
        )]
        $(#[$attrs])*
        $vis fn $name($($args)*) $(-> $ret)?;
    };
}

// Keep this list in sync with `_Py*` function imports declared through
// `extern_libpython!`. The x86 workaround only needs to apply to functions:
// statics keep their original names even when `import_name_type` is set. Match
// by name only here so the function signature stays in a single generic arm.
//
// TODO: reduce the number of `_Py*` exports from pyo3-ffi over time — the fewer
// CPython-private functions we expose, the smaller this workaround list becomes.
#[allow(unused_macros, reason = "used indirectly by extern_libpython_item!")]
macro_rules! extern_libpython_maybe_private_fn {
    (
        [_PyObject_CallFunction_SizeT]
        $(#[$attrs:meta])* $vis:vis fn $name:ident($($args:tt)*) $(-> $ret:ty)?
    ) => {
        extern_libpython_cpython_private_fn! { $(#[$attrs])* $vis $name($($args)*) $(-> $ret)? }
    };
    (
        [_PyObject_MakeTpCall]
        $(#[$attrs:meta])* $vis:vis fn $name:ident($($args:tt)*) $(-> $ret:ty)?
    ) => {
        extern_libpython_cpython_private_fn! { $(#[$attrs])* $vis $name($($args)*) $(-> $ret)? }
    };
    (
        [_Py_CheckFunctionResult]
        $(#[$attrs:meta])* $vis:vis fn $name:ident($($args:tt)*) $(-> $ret:ty)?
    ) => {
        extern_libpython_cpython_private_fn! { $(#[$attrs])* $vis $name($($args)*) $(-> $ret)? }
    };
    (
        [_PyBytes_Resize]
        $(#[$attrs:meta])* $vis:vis fn $name:ident($($args:tt)*) $(-> $ret:ty)?
    ) => {
        extern_libpython_cpython_private_fn! { $(#[$attrs])* $vis $name($($args)*) $(-> $ret)? }
    };
    (
        [_PyLong_AsByteArray]
        $(#[$attrs:meta])* $vis:vis fn $name:ident($($args:tt)*) $(-> $ret:ty)?
    ) => {
        extern_libpython_cpython_private_fn! { $(#[$attrs])* $vis $name($($args)*) $(-> $ret)? }
    };
    (
        [_PyLong_FromByteArray]
        $(#[$attrs:meta])* $vis:vis fn $name:ident($($args:tt)*) $(-> $ret:ty)?
    ) => {
        extern_libpython_cpython_private_fn! { $(#[$attrs])* $vis $name($($args)*) $(-> $ret)? }
    };
    (
        [_PyUnicode_Ready]
        $(#[$attrs:meta])* $vis:vis fn $name:ident($($args:tt)*) $(-> $ret:ty)?
    ) => {
        extern_libpython_cpython_private_fn! { $(#[$attrs])* $vis $name($($args)*) $(-> $ret)? }
    };
    (
        [_PyUnicode_ToDecimalDigit]
        $(#[$attrs:meta])* $vis:vis fn $name:ident($($args:tt)*) $(-> $ret:ty)?
    ) => {
        extern_libpython_cpython_private_fn! { $(#[$attrs])* $vis $name($($args)*) $(-> $ret)? }
    };
    (
        [_PyThreadState_UncheckedGet]
        $(#[$attrs:meta])* $vis:vis fn $name:ident($($args:tt)*) $(-> $ret:ty)?
    ) => {
        extern_libpython_cpython_private_fn! { $(#[$attrs])* $vis $name($($args)*) $(-> $ret)? }
    };
    (
        [_PyInterpreterState_GetEvalFrameFunc]
        $(#[$attrs:meta])* $vis:vis fn $name:ident($($args:tt)*) $(-> $ret:ty)?
    ) => {
        extern_libpython_cpython_private_fn! { $(#[$attrs])* $vis $name($($args)*) $(-> $ret)? }
    };
    (
        [_PyInterpreterState_SetEvalFrameFunc]
        $(#[$attrs:meta])* $vis:vis fn $name:ident($($args:tt)*) $(-> $ret:ty)?
    ) => {
        extern_libpython_cpython_private_fn! { $(#[$attrs])* $vis $name($($args)*) $(-> $ret)? }
    };
    (
        [_PyObject_GC_New]
        $(#[$attrs:meta])* $vis:vis fn $name:ident($($args:tt)*) $(-> $ret:ty)?
    ) => {
        extern_libpython_cpython_private_fn! { $(#[$attrs])* $vis $name($($args)*) $(-> $ret)? }
    };
    (
        [_PyObject_GC_NewVar]
        $(#[$attrs:meta])* $vis:vis fn $name:ident($($args:tt)*) $(-> $ret:ty)?
    ) => {
        extern_libpython_cpython_private_fn! { $(#[$attrs])* $vis $name($($args)*) $(-> $ret)? }
    };
    (
        [_PyObject_GC_Resize]
        $(#[$attrs:meta])* $vis:vis fn $name:ident($($args:tt)*) $(-> $ret:ty)?
    ) => {
        extern_libpython_cpython_private_fn! { $(#[$attrs])* $vis $name($($args)*) $(-> $ret)? }
    };
    (
        [_PyObject_New]
        $(#[$attrs:meta])* $vis:vis fn $name:ident($($args:tt)*) $(-> $ret:ty)?
    ) => {
        extern_libpython_cpython_private_fn! { $(#[$attrs])* $vis $name($($args)*) $(-> $ret)? }
    };
    (
        [_PyObject_NewVar]
        $(#[$attrs:meta])* $vis:vis fn $name:ident($($args:tt)*) $(-> $ret:ty)?
    ) => {
        extern_libpython_cpython_private_fn! { $(#[$attrs])* $vis $name($($args)*) $(-> $ret)? }
    };
    (
        [_Py_HashBytes]
        $(#[$attrs:meta])* $vis:vis fn $name:ident($($args:tt)*) $(-> $ret:ty)?
    ) => {
        extern_libpython_cpython_private_fn! { $(#[$attrs])* $vis $name($($args)*) $(-> $ret)? }
    };
    (
        [_Py_DECREF_DecRefTotal]
        $(#[$attrs:meta])* $vis:vis fn $name:ident($($args:tt)*) $(-> $ret:ty)?
    ) => {
        extern_libpython_cpython_private_fn! { $(#[$attrs])* $vis $name($($args)*) $(-> $ret)? }
    };
    (
        [_Py_Dealloc]
        $(#[$attrs:meta])* $vis:vis fn $name:ident($($args:tt)*) $(-> $ret:ty)?
    ) => {
        extern_libpython_cpython_private_fn! { $(#[$attrs])* $vis $name($($args)*) $(-> $ret)? }
    };
    (
        [_Py_DecRef]
        $(#[$attrs:meta])* $vis:vis fn $name:ident($($args:tt)*) $(-> $ret:ty)?
    ) => {
        extern_libpython_cpython_private_fn! { $(#[$attrs])* $vis $name($($args)*) $(-> $ret)? }
    };
    (
        [_Py_INCREF_IncRefTotal]
        $(#[$attrs:meta])* $vis:vis fn $name:ident($($args:tt)*) $(-> $ret:ty)?
    ) => {
        extern_libpython_cpython_private_fn! { $(#[$attrs])* $vis $name($($args)*) $(-> $ret)? }
    };
    (
        [_Py_IncRef]
        $(#[$attrs:meta])* $vis:vis fn $name:ident($($args:tt)*) $(-> $ret:ty)?
    ) => {
        extern_libpython_cpython_private_fn! { $(#[$attrs])* $vis $name($($args)*) $(-> $ret)? }
    };
    (
        [_Py_NegativeRefcount]
        $(#[$attrs:meta])* $vis:vis fn $name:ident($($args:tt)*) $(-> $ret:ty)?
    ) => {
        extern_libpython_cpython_private_fn! { $(#[$attrs])* $vis $name($($args)*) $(-> $ret)? }
    };
    (
        [_PyErr_BadInternalCall]
        $(#[$attrs:meta])* $vis:vis fn $name:ident($($args:tt)*) $(-> $ret:ty)?
    ) => {
        extern_libpython_cpython_private_fn! { $(#[$attrs])* $vis $name($($args)*) $(-> $ret)? }
    };
    (
        [$name:ident]
        $(#[$attrs:meta])* $vis:vis fn $fn_name:ident($($args:tt)*) $(-> $ret:ty)?
    ) => {
        $(#[$attrs])*
        $vis fn $fn_name($($args)*) $(-> $ret)?;
    };
}

macro_rules! extern_libpython_item {
    ($(#[$attrs:meta])* $vis:vis fn $name:ident($($args:tt)*) $(-> $ret:ty)?) => {
        extern_libpython_maybe_private_fn! {
            [$name]
            $(#[$attrs])*
            $vis fn $name($($args)*) $(-> $ret)?
        }
    };
    ($(#[$attrs:meta])* $vis:vis static mut $name:ident: $ty:ty) => {
        $(#[$attrs])*
        $vis static mut $name: $ty;
    };
    ($(#[$attrs:meta])* $vis:vis static $name:ident: $ty:ty) => {
        $(#[$attrs])*
        $vis static $name: $ty;
    };
}

macro_rules! extern_libpython_items {
    () => {};
    (
        $(#[$attrs:meta])*
        $vis:vis fn $name:ident($($args:tt)*) $(-> $ret:ty)?;
        $($rest:tt)*
    ) => {
        extern_libpython_item! {
            $(#[$attrs])*
            $vis fn $name($($args)*) $(-> $ret)?
        }
        extern_libpython_items! { $($rest)* }
    };
    (
        $(#[$attrs:meta])*
        $vis:vis static mut $name:ident: $ty:ty;
        $($rest:tt)*
    ) => {
        extern_libpython_item! {
            $(#[$attrs])*
            $vis static mut $name: $ty
        }
        extern_libpython_items! { $($rest)* }
    };
    (
        $(#[$attrs:meta])*
        $vis:vis static $name:ident: $ty:ty;
        $($rest:tt)*
    ) => {
        extern_libpython_item! {
            $(#[$attrs])*
            $vis static $name: $ty
        }
        extern_libpython_items! { $($rest)* }
    };
}

// Helper macro to declare `extern` blocks that link against libpython on Windows
// using `raw-dylib`, eliminating the need for import libraries.
//
// The target DLL name is detected at build time, but `#[link(name = "...")]`
// requires a string literal, so a specialized macro definition is generated:
//
// - Cargo builds: `pyo3-ffi/build.rs` writes it to `$OUT_DIR/extern_libpython_link.rs`.
// - Meson builds: `pyo3-ffi/meson.build` writes it to `$OUT_DIR` when `OUT_DIR`
//   is provided by the build environment.
//
// Both set the `pyo3_dll_codegen` cfg to include the generated definition below.
// This supports any DLL name, including future Python versions, without a
// hard-coded list of names.
//
// # Usage
//
// ```rust,ignore
// // Default ABI "C" (most common):
// extern_libpython! {
//     pub fn PyObject_Call(
//         callable: *mut PyObject,
//         args: *mut PyObject,
//         kwargs: *mut PyObject,
//     ) -> *mut PyObject;
// }
//
// // Explicit ABI:
// extern_libpython! { "C-unwind" {
//     pub fn PyGILState_Ensure() -> PyGILState_STATE;
// }}
// ```
#[cfg(pyo3_dll_codegen)]
include!(concat!(env!("OUT_DIR"), "/extern_libpython_link.rs"));

/// Helper macro to declare `extern` blocks that link against libpython on Windows
/// using `raw-dylib`, eliminating the need for import libraries.
///
/// This is the fallback definition used when `pyo3_dll_codegen` is not set:
/// non-Windows builds emit no `#[link]` attribute, and Windows builds without
/// raw-dylib linking link the `pythonXY` alias (renamed by the build script).
#[cfg(not(pyo3_dll_codegen))]
macro_rules! extern_libpython {
    // Explicit ABI
    ($abi:literal { $($body:tt)* }) => {
        #[cfg_attr(all(windows, not(pyo3_use_raw_dylib)), link(name = "pythonXY"))]
        unsafe extern $abi {
            extern_libpython_items! { $($body)* }
        }
    };
    // Default ABI: "C"
    ($($body:tt)*) => {
        extern_libpython!("C" { $($body)* });
    };
}
