use pyo3_build_config::{
    InterpreterConfig, PythonAbiKind, PythonImplementation, PythonVersion, StableAbi, bail, ensure,
    pyo3_build_script_impl::{
        BuildConfig, BuildConfigSource, cargo_env_var, env_var, errors::Result,
        is_linking_libpython_for_target, print_feature_cfgs, print_libpython_rpath_link_args,
        resolve_build_config, target_triple_from_env,
    },
    warn,
};

/// Minimum Python version PyO3 supports.
struct SupportedVersions {
    min: PythonVersion,
    max: PythonVersion,
}

const PY_3_9: PythonVersion = PythonVersion { major: 3, minor: 9 };
const PY_3_10: PythonVersion = PythonVersion {
    major: 3,
    minor: 10,
};
const PY_3_11: PythonVersion = PythonVersion {
    major: 3,
    minor: 11,
};
const PY_3_12: PythonVersion = PythonVersion {
    major: 3,
    minor: 12,
};
const PY_3_14: PythonVersion = PythonVersion {
    major: 3,
    minor: 14,
};
const PY_3_15: PythonVersion = PythonVersion {
    major: 3,
    minor: 15,
};

const SUPPORTED_VERSIONS_CPYTHON: SupportedVersions = SupportedVersions {
    min: PY_3_9,
    max: PY_3_15,
};

const SUPPORTED_VERSIONS_PYPY: SupportedVersions = SupportedVersions {
    min: PY_3_11,
    max: SUPPORTED_VERSIONS_CPYTHON.max,
};

const SUPPORTED_VERSIONS_GRAALPY: SupportedVersions = SupportedVersions {
    min: PY_3_10,
    max: SUPPORTED_VERSIONS_CPYTHON.max,
};

const MIN_FREE_THREADED_VERSION: PythonVersion = PY_3_14;

fn ensure_python_version(interpreter_config: &InterpreterConfig) -> Result<()> {
    // This is an undocumented env var which is only really intended to be used in CI / for testing
    // and development.
    if std::env::var("UNSAFE_PYO3_SKIP_VERSION_CHECK").as_deref() == Ok("1") {
        return Ok(());
    }

    let implementation = interpreter_config.target_abi().implementation();

    let supported_versions = match implementation {
        PythonImplementation::CPython => SUPPORTED_VERSIONS_CPYTHON,
        PythonImplementation::PyPy => SUPPORTED_VERSIONS_PYPY,
        PythonImplementation::GraalPy => SUPPORTED_VERSIONS_GRAALPY,
        PythonImplementation::RustPython => {
            // We don't yet check anything for RustPython
            return Ok(());
        }
    };

    let interp_version = interpreter_config.target_abi().version();
    ensure!(
        interpreter_config.version() >= supported_versions.min,
        "the configured Python interpreter version ({}) is lower than PyO3's minimum supported version ({})",
        interpreter_config.version(),
        supported_versions.min,
    );
    let expected_next_version = PythonVersion {
        major: supported_versions.max.major,
        minor: supported_versions.max.minor + 1,
    };
    if interp_version == expected_next_version {
        warn!(
            "Using experimental support for the {implementation} {}.{} ABI. \
             Build artifacts may not be compatible with the final release of {implementation}, \
             so do not distribute them.",
            expected_next_version.major, expected_next_version.minor,
        );
    } else if interp_version > expected_next_version {
        // The maximum version is not checked: future Python versions are
        // allowed to build (with a warning), so that PyO3 does not need to be
        // updated for every new Python release.
        warn!(
            "The configured {implementation} {interp_version} is newer than PyO3's maximum \
             supported version ({}); this version has not been tested with PyO3 and the \
             build artifacts may not work correctly.",
            supported_versions.max,
        );
    }

    if interpreter_config.target_abi().kind().is_free_threaded() {
        ensure!(
            implementation == PythonImplementation::CPython,
            "PyO3 only supports the free-threaded build of CPython, \
             the selected Python implementation is {implementation}",
        );
        ensure!(
            interpreter_config.target_abi().version() >= MIN_FREE_THREADED_VERSION,
            "PyO3 does not support the free-threaded build of CPython versions \
             below {MIN_FREE_THREADED_VERSION}, the selected Python version is {}",
            interpreter_config.target_abi().version(),
        );
    }

    if let PythonAbiKind::Stable(abi) = interpreter_config.target_abi().kind() {
        match implementation {
            PythonImplementation::CPython | PythonImplementation::PyPy => match abi {
                StableAbi::Abi3t => {
                    ensure!(
                        interpreter_config.target_abi().version() >= PY_3_15,
                        "Abi3t builds are not supported on CPython targets before Python 3.15"
                    )
                }
                StableAbi::Abi3 => {
                    if implementation == PythonImplementation::PyPy
                        && interpreter_config.target_abi().version() < PY_3_12
                    {
                        warn!(
                            "PyPy does not support abi3 before PyPy 3.12 so the build artifacts will be version-specific."
                        )
                    }
                }
            },
            PythonImplementation::GraalPy => warn!(
                "GraalPy does not support {abi} so the build artifacts will be version-specific."
            ),
            // NB RustPython likely to support abi3t, but no checking here yet.
            PythonImplementation::RustPython => {}
        }
    }

    Ok(())
}

fn ensure_target_pointer_width(interpreter_config: &InterpreterConfig) -> Result<()> {
    if let Some(pointer_width) = interpreter_config.pointer_width() {
        // Try to check whether the target architecture matches the python library
        let rust_target = match cargo_env_var("CARGO_CFG_TARGET_POINTER_WIDTH")
            .unwrap()
            .as_str()
        {
            "64" => 64,
            "32" => 32,
            x => bail!("unexpected Rust target pointer width: {}", x),
        };

        ensure!(
            rust_target == pointer_width,
            "your Rust target architecture ({}-bit) does not match your python interpreter ({}-bit)",
            rust_target,
            pointer_width
        );
    }
    Ok(())
}

/// Whether to use raw-dylib linking.
///
/// Currently, this only applies if the target OS is Windows and the
/// `PYO3_USE_RAW_DYLIB` environment variable is not set, or is set to `1`.
///
/// The Python library name does not need to be known in advance: the build
/// script generates an `extern_libpython!` macro specialized to the detected
/// DLL name, so `raw-dylib` works for any name (including future Python
/// versions) without a hard-coded list.
///
/// NB in some cases (e.g. mixed C / Rust builds) it might be necessary to link the full Python
/// library rather than rely on the symbols which PyO3 defines as raw-dylib, which is why
/// we have the opt-out env var.
fn should_use_raw_dylib_linking() -> bool {
    if cargo_env_var("CARGO_CFG_TARGET_OS").unwrap() != "windows" {
        return false;
    }
    env_var("PYO3_USE_RAW_DYLIB").is_none_or(|os_str| os_str == "1")
}

/// Writes an `extern_libpython!` macro definition specialized to `lib_name` into
/// `OUT_DIR`, so that the `#[link(name = "...", kind = "raw-dylib")]` attribute can
/// use the automatically detected DLL name instead of matching against a hard-coded
/// list of known names in `pyo3-ffi/src/impl_/macros.rs`.
///
/// `#[link]` requires a string literal, so the name cannot be passed via a cfg
/// value; generating the macro is the only way to avoid the hard-coded list.
/// Meson builds can generate the same file (see `pyo3-ffi/meson.build`); builds
/// which provide no `OUT_DIR` fall back to the static macro definition.
fn emit_extern_libpython_macro(lib_name: &str) {
    assert!(
        !lib_name.is_empty()
            && lib_name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')),
        "unexpected characters in Python library name `{lib_name}`"
    );
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR is always set for build scripts");
    let path = std::path::Path::new(&out_dir).join("extern_libpython_link.rs");
    std::fs::write(
        &path,
        format!(
            r#"macro_rules! extern_libpython {{
    // Explicit ABI
    ($abi:literal {{ $($body:tt)* }}) => {{
        #[cfg_attr(
            all(windows, pyo3_use_raw_dylib, target_arch = "x86"),
            link(name = "{lib_name}", kind = "raw-dylib", import_name_type = "undecorated")
        )]
        #[cfg_attr(
            all(windows, pyo3_use_raw_dylib, not(target_arch = "x86")),
            link(name = "{lib_name}", kind = "raw-dylib")
        )]
        #[cfg_attr(all(windows, not(pyo3_use_raw_dylib)), link(name = "pythonXY"))]
        unsafe extern $abi {{
            extern_libpython_items! {{ $($body)* }}
        }}
    }};
    // Default ABI: "C"
    ($($body:tt)*) => {{
        extern_libpython!("C" {{ $($body)* }});
    }};
}}
"#
        ),
    )
    .expect("failed to write extern_libpython_link.rs");
    println!("cargo:rustc-cfg=pyo3_dll_codegen");
}

fn emit_link_config(build_config: &BuildConfig) -> Result<()> {
    let target_os = cargo_env_var("CARGO_CFG_TARGET_OS").unwrap();
    let interpreter_config = &build_config.interpreter_config;

    let lib_name = interpreter_config
        .lib_name()
        .ok_or("attempted to link to Python shared library but config does not contain lib_name")?;

    if should_use_raw_dylib_linking() {
        // Use raw-dylib linking: generate an `extern_libpython!` macro specialized to
        // the detected DLL name so that it expands to the right
        // `#[link(name = "...", kind = "raw-dylib")]` attribute at compile time.
        // This eliminates the need for import libraries (.lib files) entirely.
        //
        // Note: raw-dylib is inherently dynamic linking. Static embedding of the
        // Python interpreter on Windows is not supported by this path (and is not
        // officially supported by CPython on Windows).
        println!("cargo:rustc-cfg=pyo3_use_raw_dylib");
        emit_extern_libpython_macro(lib_name);
        return Ok(());
    }

    // Not using raw-dylib linking: PyPy dll needs to be the import library not the DLL name
    let lib_name = if interpreter_config.target_abi().implementation() == PythonImplementation::PyPy
        && target_os == "windows"
    {
        // FIXME: this should probably be done with better configuration in pyo3-build-config
        // for `raw-dylib` in general, rather than as a patch here.
        //
        // Assert expected raw pypy dll name as a sanity check for now
        assert_eq!(
            lib_name,
            format!(
                "libpypy3.{}-c",
                interpreter_config.target_abi().version().minor
            )
        );
        format!(
            "python{}{}",
            interpreter_config.target_abi().version().major,
            interpreter_config.target_abi().version().minor
        )
    } else {
        lib_name.to_string()
    };

    println!(
        "cargo:rustc-link-lib={link_model}{alias}{lib_name}",
        link_model = if interpreter_config.shared() {
            ""
        } else {
            "static="
        },
        // on windows we emit `#[link(name = "pythonXY")]` attributes
        // and need this alias here to get the right name for the final link
        alias = if target_os == "windows" {
            "pythonXY:"
        } else {
            ""
        }
    );

    if let Some(lib_dir) = interpreter_config.lib_dir() {
        println!("cargo:rustc-link-search=native={lib_dir}");
    } else if matches!(build_config.source, BuildConfigSource::CrossCompile) {
        warn!(
            "The output binary will link to libpython, \
            but PYO3_CROSS_LIB_DIR environment variable is not set. \
            Ensure that the target Python library directory is \
            in the rustc native library search path."
        );
    }

    Ok(())
}

/// Prepares the PyO3 crate for compilation.
///
/// This uses pyo3-build-config implementation to detect the target Python interpreter and validate
/// it's suitable for building with.
///
/// Emits the cargo configuration based on this config as well as a few checks of the Rust compiler
/// version to enable features which aren't supported on MSRV.
fn configure_pyo3_ffi() -> Result<()> {
    let target = target_triple_from_env();
    let build_config = resolve_build_config(&target)?;
    let interpreter_config = &build_config.interpreter_config;

    if env_var("PYO3_PRINT_CONFIG").is_some_and(|os_str| os_str == "1") {
        print_config_and_exit(interpreter_config);
    }

    ensure_python_version(interpreter_config)?;
    ensure_target_pointer_width(interpreter_config)?;

    // Serialize the whole interpreter config into DEP_PYTHON_PYO3_CONFIG env var.
    interpreter_config.to_cargo_dep_env()?;

    if is_linking_libpython_for_target(&target)
        && !interpreter_config.suppress_build_script_link_lines()
    {
        emit_link_config(&build_config)?;
    }

    for cfg in interpreter_config.build_script_outputs() {
        println!("{cfg}")
    }

    // Extra lines come last, to support last write wins.
    for line in interpreter_config.extra_build_script_lines() {
        println!("{line}");
    }

    print_feature_cfgs();

    // Make `cargo test` etc work with non-system Python installations
    print_libpython_rpath_link_args(&target, interpreter_config);

    Ok(())
}

fn print_config_and_exit(config: &InterpreterConfig) {
    println!("\n-- PYO3_PRINT_CONFIG=1 is set, printing configuration and halting compile --");
    config
        .to_writer(std::io::stdout())
        .expect("failed to print config to stdout");
    println!(
        "\nnote: unset the PYO3_PRINT_CONFIG environment variable and retry to compile with the above config"
    );
    std::process::exit(101);
}

fn main() {
    pyo3_build_config::print_expected_cfgs();
    if let Err(e) = configure_pyo3_ffi() {
        eprintln!("error: {}", e.report());
        std::process::exit(1)
    }
}
