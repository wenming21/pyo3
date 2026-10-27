project(
  'pyo3-ffi',
  'rust',
  meson_version : '>= 1.12',
  version : '0.29.2',
  license : 'MIT OR Apache-2.0'
)

MINIMUM_SUPPORTED_VERSION_MINOR = 8
STABLE_ABI_MAX_MINOR = 15
# Margin above the maximum supported version (not checked at build time).
PY_3_CFG_FUTURE_MARGIN = 15
MINIMUM_SUPPORTED_VERSION_PYPY_MINOR = 11
MAXIMUM_SUPPORTED_VERSION_PYPY_MINOR = 11

extra_args = []

# Registers PyO3's config names as reachable cfg expressions.
extra_args += [
  '--check-cfg', 'cfg(Py_LIMITED_API)',
  '--check-cfg', 'cfg(Py_GIL_DISABLED)',
  '--check-cfg', 'cfg(PyPy)',
  '--check-cfg', 'cfg(GraalPy)',
  '--check-cfg', 'cfg(RustPython)',
  '--check-cfg', 'cfg(py_sys_config, values("Py_DEBUG", "Py_REF_DEBUG", "Py_TRACE_REFS", "COUNT_ALLOCS"))',
  # Set when a specialized `extern_libpython!` macro is generated (see below).
  '--check-cfg', 'cfg(pyo3_dll_codegen)',
]

# Allow 'Py_3_{minor}' cfgs from the minimum supported version up to
# the maximum minor version (+1 for development for the next), plus a
# margin for future versions which are not checked at build time.
foreach i : range(MINIMUM_SUPPORTED_VERSION_MINOR, STABLE_ABI_MAX_MINOR + PY_3_CFG_FUTURE_MARGIN + 2)
  extra_args += ['--check-cfg', f'cfg(Py_3_@i@)']
endforeach




fs = import('fs')
rust = import('rust')
cargo_ws = rust.workspace()
pkg_obj = cargo_ws.package('pyo3-ffi', native : false)


features = pkg_obj.features()
message('Enabled features:', features)


# Deprecated cargo features.
if features.contains('extension-module')
  warning('The extension-module cargo feature is deprecated. ')
endif
if features.contains('generate-import-lib')
  warning('The generate-import-lib cargo feature is deprecated')
endif


py = import('python').find_installation()
py_minor_version = py.language_version().split('.')[1].to_int()

is_windows = host_machine.system() == 'windows'
py_gil_disabled = py.get_variable('Py_GIL_DISABLED', 0) == 1
py_debug = py.get_variable('Py_DEBUG', 0) == 1


# Implementation detection.
implementation = run_command(
  py, '-c', 'import sys; print(sys.implementation.name)',
  check : true,
).stdout().strip()
is_pypy = implementation == 'pypy'
is_graalpy = implementation == 'graalpy'
if is_pypy
  extra_args += ['--cfg', 'PyPy']
elif is_graalpy
  extra_args += ['--cfg', 'GraalPy']
elif implementation != 'cpython'
  error(f'Unsupported Python implementation: @implementation@')
endif


# Python ABI.
if features.contains('abi3t')
  if not py_gil_disabled
    error('abi3t required but Python is not free-threaded')
  endif
  abi = '3t'
  limited_api = true
elif features.contains('abi3')
  if py_gil_disabled
    error('abi3 required but Python is free-threaded')
  endif
  abi = '3'
  limited_api = true
else
  abi = f'3@py_minor_version@' + (py_gil_disabled ? 't' : '')
  limited_api = false
endif

if py_gil_disabled
  extra_args += ['--cfg', 'Py_GIL_DISABLED']
endif


# Emit cumulative 'Py_3_{minor}' cfgs up to the interpreter version,
# or up to the abi3 floor when targeting the stable ABI.
py_minor_version_max = py_minor_version
if limited_api
  if is_pypy or is_graalpy
    error('abi3 builds on PyPy/GraalPy are not supported yet')
  endif

  extra_args += ['--cfg', 'Py_LIMITED_API']
  # Parse feature in the form 'abi3-py3{minor}' or 'abi3t-py3{minor}'
  # to extract the minor version.
  foreach f : features
    if f.startswith('abi3-py3')
      floor = f.substring(8).to_int()
    elif f.startswith('abi3t-py3')
      floor = f.substring(9).to_int()
    else
      continue
    endif
    py_minor_version_max = floor < py_minor_version_max ? floor : py_minor_version_max
  endforeach
endif

foreach m : range(MINIMUM_SUPPORTED_VERSION_MINOR, py_minor_version_max + 1)
  extra_args += ['--cfg', f'Py_3_@m@']
endforeach


foreach flag : ['Py_DEBUG', 'Py_REF_DEBUG', 'Py_TRACE_REFS', 'COUNT_ALLOCS']
  if py.get_variable(flag, 0) == 1
    extra_args += ['--cfg', f'py_sys_config="@flag@"']
  endif
endforeach


if is_windows
  # Windows: extension modules need to link to libpython.
  if is_pypy
    py_lib_name = f'libpypy3.@py_minor_version@-c'
  elif is_graalpy
    py_lib_name = 'python-native'
  else
    py_lib_name = 'python' + abi + (py_debug ? '_d' : '')
  endif
  extra_args += ['--cfg', 'pyo3_use_raw_dylib', '--cfg', 'pyo3_dll_codegen']

  # The DLL name is detected above, but `#[link(name = "...")]` needs a string
  # literal, so it cannot be passed via a cfg value. Generate a specialized
  # `extern_libpython!` macro into `OUT_DIR` (which rustc reads via
  # `env!("OUT_DIR")`), so that no hard-coded list of known DLL names is needed
  # and future Python versions work without changes.
  #
  # Cargo sets `OUT_DIR` itself; for Meson, the build environment must provide
  # it (the meson-python build backend in `examples/meson-starter` does).
  out_dir = run_command(
    py, '-c', 'import os; print(os.environ.get("OUT_DIR", ""))',
    check : false,
  ).stdout().strip()
  if out_dir == ''
    error('pyo3-ffi Meson builds on Windows require the OUT_DIR environment variable ' +
          'so that the detected Python DLL name can be written to the generated ' +
          '`extern_libpython!` macro. Cargo sets this automatically; Meson build ' +
          'environments (e.g. meson-python backends) must set it too.')
  endif
  configure_file(
    input : 'extern_libpython_link.rs.in',
    output : 'extern_libpython_link.rs',
    configuration : {'PYO3_DLL_NAME' : py_lib_name},
  )
  run_command(
    py, '-c',
    'import os, shutil, sys; os.makedirs(sys.argv[2], exist_ok=True); shutil.copyfile(sys.argv[1], os.path.join(sys.argv[2], "extern_libpython_link.rs"))',
    join_paths(meson.current_build_dir(), 'extern_libpython_link.rs'), out_dir,
    check : true,
  )
endif


# Compiler features used by PyO3.
rustc_features = {
  'const_is_null': 84,
  'fn_ptr_eq': 85,
  'from_bytes_with_nul_error': 86,
  'cfg_select': 95,
}
rustc_minor_version = meson.get_compiler('rust').version().split('.')[1].to_int()
foreach cfg, minor_version_required : rustc_features
  extra_args += ['--check-cfg', f'cfg(@cfg@)']
  if rustc_minor_version >= minor_version_required
    extra_args += ['--cfg', cfg]
  endif
endforeach


lib = pkg_obj.library('pyo3_ffi', rust_args : extra_args)
dep = declare_dependency(
  link_with : lib,
  variables : {
    'features' : ','.join(features),
    'args' : '\0'.join(extra_args),
  },
  version : pkg_obj.version()
)
pkg_obj.override_dependency(dep)
