project(
  'pyo3-ffi',
  'rust',
  meson_version : '>= 1.12',
  version : '0.30.0',
  license : 'MIT OR Apache-2.0'
)

MINIMUM_SUPPORTED_VERSION_MINOR = 9
STABLE_ABI_MAX_MINOR = 15
MINIMUM_SUPPORTED_VERSION_PYPY_MINOR = 11
MAXIMUM_SUPPORTED_VERSION_PYPY_MINOR = 13

extra_args = []

# Registers PyO3's config names as reachable cfg expressions.
extra_args += [
  '--check-cfg', 'cfg(Py_LIMITED_API)',
  '--check-cfg', 'cfg(Py_GIL_DISABLED)',
  '--check-cfg', 'cfg(PyPy)',
  '--check-cfg', 'cfg(GraalPy)',
  '--check-cfg', 'cfg(RustPython)',
  '--check-cfg', 'cfg(pyo3_use_auto_detect_lib)',
  '--check-cfg', 'cfg(pyo3_auto_detect_out_dir, values("out-dir", "meson"))',
  '--check-cfg', 'cfg(py_sys_config, values("Py_DEBUG", "Py_REF_DEBUG", "Py_TRACE_REFS", "COUNT_ALLOCS"))',
]

# Allow 'Py_3_{minor}' cfgs from the minimum supported version up to
# the maximum minor version (+1 for development for the next).
foreach i : range(MINIMUM_SUPPORTED_VERSION_MINOR, STABLE_ABI_MAX_MINOR + 2)
  extra_args += ['--check-cfg', f'cfg(Py_3_@i@)']
endforeach

# Allow all possible 'pyo3_dll' cfg values for raw-dylib linking on Windows.
dll_names = ['python3', 'python3_d', 'python3t', 'python3t_d']

foreach i : range(MINIMUM_SUPPORTED_VERSION_MINOR, STABLE_ABI_MAX_MINOR + 2)
  # CPython DLL names.
  dll_names += [f'python3@i@', f'python3@i@_d']
  if i >= 13
    dll_names += [f'python3@i@t', f'python3@i@t_d']
  endif
endforeach

foreach i : range(MINIMUM_SUPPORTED_VERSION_PYPY_MINOR, MAXIMUM_SUPPORTED_VERSION_PYPY_MINOR + 1)
  # PyPy DLL names.
  dll_names += [f'libpypy3.@i@-c']
endforeach

# GraalPy DLL name
dll_names += ['python-native']

dll_names_quoted = []
foreach name : dll_names
  dll_names_quoted += f'"@name@"'
endforeach

pyo3_dll_values = ', '.join(dll_names_quoted)
extra_args += ['--check-cfg', f'cfg(pyo3_dll, values(@pyo3_dll_values@))']



rust = import('rust')
cargo_ws = rust.workspace()
pkg_obj = cargo_ws.package('pyo3-ffi', native : false)


features = pkg_obj.features()
message('Enabled features:', features)

auto_detect = features.contains('auto-detect-python-lib')


# Deprecated cargo features.
if features.contains('extension-module')
  warning('The extension-module cargo feature is deprecated. ')
endif
if features.contains('generate-import-lib')
  if auto_detect
    warning('[pyo3] auto-detect-python-lib is enabled; generate-import-lib is disabled and no import library will be generated')
  else
    warning('The generate-import-lib cargo feature is deprecated')
  endif
endif


py = import('python').find_installation()
py_minor_version = py.language_version().split('.')[1].to_int()

is_windows = host_machine.system() == 'windows'
py_gil_disabled = py.get_variable('Py_GIL_DISABLED', 0) == 1
py_debug = py.get_variable('Py_DEBUG', 0) == 1

rustc = find_program('rustc')
rustc_host_triple = ''
foreach line : run_command(rustc, '-vV', check : true).stdout().split('\n')
  if line.startswith('host:')
    rustc_host_triple = line.split(':')[1].strip()
  endif
endforeach
is_mingw = is_windows and rustc_host_triple.contains('-gnu')


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


carrier_link_with = []

if is_windows
  # Windows: extension modules need to link to libpython.
  free_threaded_suffix = py_gil_disabled ? 't' : ''
  debug_suffix = py_debug ? '_d' : ''
  if is_pypy
    py_lib_name = f'libpypy3.@py_minor_version@-c'
  elif is_graalpy
    py_lib_name = 'python-native'
  elif is_mingw
    # MinGW uses Unix-style names with a `lib` prefix.
    if limited_api
      py_lib_name = 'libpython3' + free_threaded_suffix + (py_debug ? 'd' : '')
    else
      py_lib_name = f'libpython3.@py_minor_version@' + free_threaded_suffix + (py_debug ? 'd' : '')
    endif
  elif py_debug and py_minor_version < 10
    # CPython bug: linking against python3_d.dll raises error
    # https://github.com/python/cpython/issues/101614
    py_lib_name = f'python3@py_minor_version@_d'
  elif limited_api
    py_lib_name = 'python3' + free_threaded_suffix + debug_suffix
  else
    py_lib_name = f'python3@py_minor_version@' + free_threaded_suffix + debug_suffix
  endif

  if auto_detect
    message(f'[pyo3] auto-detect-python-lib: lib_name=@py_lib_name@ tool=meson source=detected')
    if dll_names.contains(py_lib_name)
      # Inline channel: reuse the existing literal candidate list.
      extra_args += [
        '--cfg', 'pyo3_use_raw_dylib',
        '--cfg', f'pyo3_dll="@py_lib_name@"',
        '--cfg', 'pyo3_use_auto_detect_lib',
      ]
    else
      # Link attribute carrier: a small Rust crate is generated in the build
      # directory and linked into pyo3-ffi. `py` only writes the source file;
      # no additional interpreter probe is performed.
      if host_machine.cpu_family() == 'x86'
        link_attr = f'#[link(name = "@py_lib_name@", kind = "raw-dylib", import_name_type = "undecorated")]'
      else
        link_attr = f'#[link(name = "@py_lib_name@", kind = "raw-dylib")]'
      endif
      carrier_script = '''
import pathlib, sys
path = pathlib.Path(sys.argv[1])
link_attr = sys.argv[2]
content = """#[macro_export]
macro_rules! pyo3_auto_detect_extern {
    ($abi:literal { $($body:tt)* }) => {
        LINK_ATTRIBUTE
        unsafe extern $abi { extern_libpython_items! { $($body)* } }
    };
}
"""
content = content.replace("LINK_ATTRIBUTE", link_attr)
path.write_text(content, encoding="utf-8")
'''
      carrier_rs = custom_target(
        'pyo3_auto_detect_lib_rs',
        output : 'pyo3_auto_detect_lib.rs',
        command : [py, '-c', carrier_script, '@OUTPUT@', link_attr],
      )
      carrier_lib = static_library(
        'pyo3_auto_detect_lib',
        carrier_rs,

        rust_crate_type : 'rlib',
      )
      extra_args += [
        '--cfg', 'pyo3_use_auto_detect_lib',
        '--cfg', 'pyo3_auto_detect_out_dir="meson"',
      ]
      carrier_link_with += carrier_lib
    endif
  else
    extra_args += ['--cfg', 'pyo3_use_raw_dylib', '--cfg', f'pyo3_dll="@py_lib_name@"']
  endif
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


lib = pkg_obj.library('pyo3_ffi', rust_args : extra_args, link_with : carrier_link_with)
dep = declare_dependency(
  link_with : lib,
  variables : {
    'features' : ','.join(features),
    'args' : '\0'.join(extra_args),
  },
  version : pkg_obj.version()
)
pkg_obj.override_dependency(dep)
