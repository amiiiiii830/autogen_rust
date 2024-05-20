use rustpython::vm::{py_freeze, Settings};
use rustpython::InterpreterConfig;
use rustpython_vm as vm;

pub fn run_python(code: &str) {
    let interpreter = InterpreterConfig::new().init_stdlib().interpreter();
    interpreter.enter(|vm| {
        let scope = vm.new_scope_with_builtins();
        let code_obj = vm
            .compile(code, vm::compiler::Mode::Exec, "<embedded>".to_owned())
            .map_err(|err| vm.new_syntax_error(&err, Some(code)))
            .expect("msg");

        vm.run_code_obj(code_obj, scope).expect("msg");
    });
}

pub fn run_python_vm(code: &str) {
    let settings = Settings::default();
    let settings = Settings::with_path(settings, "/Users/jichen/.cargo/bin/rustpython".to_owned());
    // let settings = Settings::with_path(
    //     settings,
    //     "/Users/jichen/Downloads/RustPython-0.3.1/pylib/Lib/".to_owned(),
    // );

    vm::Interpreter::with_init(settings, |vm| {
        vm.add_native_modules(rustpython_stdlib::get_module_inits());
        vm.add_frozen(rustpython_vm::py_freeze!(
            dir = "/Users/jichen/Downloads/RustPython-0.3.1/pylib/Lib/"
        ));
    })
    .enter(|vm| {
        vm.run_code_string(vm.new_scope_with_builtins(), code, "<...>".to_owned());
    });
}

pub fn run_python_func(func_path: &str) -> anyhow::Result<String, String> {
    match std::process::Command::new("/Users/jichen/.cargo/bin/rustpython")
        .arg(func_path)
        .output()
    {
        Ok(out) => {
            if !out.stdout.is_empty() {
                Ok(format!(
                    "Output: {}",
                    String::from_utf8(out.stdout).unwrap()
                ))
            } else {
                Err("empty result".to_string())
            }
        }

        Err(e) => Err(format!("Failed to execute command: {}", e)),
    }
}

// export DYLD_LIBRARY_PATH=/Users/jichen/miniconda3/lib:$DYLD_LIBRARY_PATH
// export PYO3_PYTHON=/Users/jichen/miniconda3/bin/python
// export DYLD_LIBRARY_PATH=/Users/jichen/miniconda3/lib
