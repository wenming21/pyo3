//! Generation of the `pyo3_libpython_link!` macro used for raw-dylib linking
//! against libpython on Windows.
//!
//! The macro definition is generated at build time (by `pyo3-ffi`'s build
//! script, or by `pyo3-ffi/meson.build`) with the detected Python shared
//! library name baked into a `#[link(name = "...", kind = "raw-dylib")]`
//! attribute, so that `extern_libpython!` in `pyo3-ffi` can link to libpython
//! without the need for import libraries (`.lib` files).

/// Returns the `import_name_type` attribute value for the given target
/// architecture.
///
/// On x86, Python DLLs export undecorated symbol names (no leading
/// underscore), but the default for raw-dylib on x86 is fully-decorated
/// (cdecl adds a `_` prefix). `import_name_type = "undecorated"` is required
/// to match, and it is only valid on x86.
pub fn libpython_import_name_type(target_arch: &str) -> &'static str {
    if target_arch == "x86" {
        ", import_name_type = \"undecorated\""
    } else {
        ""
    }
}

/// Generates the content of the `pyo3_libpython_link!` macro definition file.
///
/// The macro wraps its body in an `unsafe extern $abi` block with a
/// `#[link(name = "{lib_name}", kind = "raw-dylib"{import_name_type})]`
/// attribute applied.
pub fn generate_libpython_link_macro_content(lib_name: &str, import_name_type: &str) -> String {
    let link_line =
        format!("        #[link(name = \"{lib_name}\", kind = \"raw-dylib\"{import_name_type})]");
    [
        "// @generated - do not edit.",
        "// Defines the `#[link(..., kind = \"raw-dylib\")]` attribute for the",
        "// auto-detected Python shared library, so that `extern_libpython!` can",
        "// link to it without the need for import libraries (.lib files).",
        "macro_rules! pyo3_libpython_link {",
        "    ($abi:literal { $($body:tt)* }) => {",
        &link_line,
        "        unsafe extern $abi { $($body)* }",
        "    };",
        "}",
        "",
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::{generate_libpython_link_macro_content, libpython_import_name_type};
    use crate::impl_::{
        PythonAbiBuilder, PythonImplementation, PythonVersion, default_lib_name_for_target,
    };
    use target_lexicon::Triple;

    const PY39: PythonVersion = PythonVersion { major: 3, minor: 9 };
    const PY311: PythonVersion = PythonVersion {
        major: 3,
        minor: 11,
    };
    const PY312: PythonVersion = PythonVersion {
        major: 3,
        minor: 12,
    };
    const PY313: PythonVersion = PythonVersion {
        major: 3,
        minor: 13,
    };
    const PY315: PythonVersion = PythonVersion {
        major: 3,
        minor: 15,
    };

    fn win_x64_triple() -> Triple {
        "x86_64-pc-windows-msvc".parse().unwrap()
    }

    #[test]
    fn import_name_type_only_on_x86() {
        assert_eq!(
            libpython_import_name_type("x86"),
            ", import_name_type = \"undecorated\""
        );
        assert_eq!(libpython_import_name_type("x86_64"), "");
        assert_eq!(libpython_import_name_type("aarch64"), "");
        assert_eq!(libpython_import_name_type("arm64"), "");
    }

    /// The macro content generated for each supported interpreter
    /// configuration must contain the exact expected `#[link]` attribute, so
    /// that raw-dylib linking works on Windows for any Python version or
    /// implementation.
    #[test]
    fn macro_content_matches_expected_link_attribute() {
        // (abi, target_arch, expected_lib_name, expect_undecorated);
        // `target_arch` mirrors the value of cargo's `CARGO_CFG_TARGET_ARCH`
        // environment variable ("x86" for 32-bit x86, not target-lexicon's
        // "i686").
        for (abi, target_arch, expected_lib_name, expect_undecorated) in [
            // CPython version-specific
            (
                PythonAbiBuilder::new(PythonImplementation::CPython, PY39)
                    .finalize()
                    .unwrap(),
                "x86_64",
                "python39",
                false,
            ),
            (
                PythonAbiBuilder::new(PythonImplementation::CPython, PY315)
                    .finalize()
                    .unwrap(),
                "x86_64",
                "python315",
                false,
            ),
            // CPython free-threaded
            (
                PythonAbiBuilder::new(PythonImplementation::CPython, PY313)
                    .free_threaded()
                    .finalize()
                    .unwrap(),
                "x86_64",
                "python313t",
                false,
            ),
            // CPython debug
            (
                PythonAbiBuilder::new(PythonImplementation::CPython, PY315)
                    .debug()
                    .finalize()
                    .unwrap(),
                "x86_64",
                "python315_d",
                false,
            ),
            // abi3 (stable ABI, no version in name)
            (
                PythonAbiBuilder::new(PythonImplementation::CPython, PY315)
                    .stable_abi(crate::StableAbi::Abi3)
                    .finalize()
                    .unwrap(),
                "x86_64",
                "python3",
                false,
            ),
            // abi3t
            (
                PythonAbiBuilder::new(PythonImplementation::CPython, PY315)
                    .stable_abi(crate::StableAbi::Abi3t)
                    .finalize()
                    .unwrap(),
                "x86_64",
                "python3t",
                false,
            ),
            // PyPy
            (
                PythonAbiBuilder::new(PythonImplementation::PyPy, PY311)
                    .finalize()
                    .unwrap(),
                "x86_64",
                "libpypy3.11-c",
                false,
            ),
            (
                PythonAbiBuilder::new(PythonImplementation::PyPy, PY312)
                    .finalize()
                    .unwrap(),
                "x86_64",
                "libpypy3.12-c",
                false,
            ),
            // GraalPy
            (
                PythonAbiBuilder::new(PythonImplementation::GraalPy, PY311)
                    .finalize()
                    .unwrap(),
                "x86_64",
                "python-native",
                false,
            ),
            // x86 requires import_name_type = "undecorated"
            (
                PythonAbiBuilder::new(PythonImplementation::CPython, PY315)
                    .finalize()
                    .unwrap(),
                "x86",
                "python315",
                true,
            ),
            // PyPy on x86
            (
                PythonAbiBuilder::new(PythonImplementation::PyPy, PY311)
                    .finalize()
                    .unwrap(),
                "x86",
                "libpypy3.11-c",
                true,
            ),
        ] {
            let lib_name = default_lib_name_for_target(abi, &win_x64_triple());
            assert_eq!(lib_name, expected_lib_name);

            let import_name_type = libpython_import_name_type(target_arch);
            assert_eq!(
                import_name_type.contains("undecorated"),
                expect_undecorated,
                "unexpected import_name_type for {target_arch}"
            );

            let content = generate_libpython_link_macro_content(&lib_name, import_name_type);
            let expected_line = if expect_undecorated {
                format!(
                    "        #[link(name = \"{expected_lib_name}\", kind = \"raw-dylib\", \
                     import_name_type = \"undecorated\")]"
                )
            } else {
                format!("        #[link(name = \"{expected_lib_name}\", kind = \"raw-dylib\")]")
            };
            assert!(
                content.contains(&expected_line),
                "generated content missing expected link attribute:\n{content}\n\nexpected line:\n{expected_line}"
            );
        }
    }

    /// The generated macro must be syntactically valid macro_rules syntax:
    /// balanced braces and a single macro definition.
    #[test]
    fn macro_content_is_balanced() {
        let content = generate_libpython_link_macro_content("python315", "");
        assert!(content.starts_with("//"));
        assert!(content.contains("macro_rules! pyo3_libpython_link {"));
        assert!(content.trim_end().ends_with('}'));
        assert_eq!(
            content.matches('{').count(),
            content.matches('}').count(),
            "unbalanced braces in generated macro"
        );
    }

    /// Future Python versions (not yet existing) must still work, since the
    /// library name is generated at build time rather than enumerated in the
    /// crate source.
    #[test]
    fn macro_content_supports_unreleased_versions() {
        for lib_name in ["python317", "python318t", "libpypy3.14-c", "libpypy3.15-c"] {
            let content = generate_libpython_link_macro_content(lib_name, "");
            let expected_line =
                format!("        #[link(name = \"{lib_name}\", kind = \"raw-dylib\")]");
            assert!(
                content.contains(&expected_line),
                "generated content missing expected link attribute for {lib_name}"
            );
        }
    }
}
