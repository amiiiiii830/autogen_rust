use rustpython::vm::Settings;
use rustpython_vm as vm;

pub fn run_python(code: &str) -> vm::PyResult<()> {
    let settings = Settings::default();
    let settings = Settings::with_path(settings, "/Users/jichen/.cargo/bin/rustpython".to_owned());

    let mut interpreter = vm::Interpreter::with_init(settings, |_| {});
    // let mut settings = Settings::default();
    // settings
    //     .path_list
    //     .push("/Users/jichen/.cargo/bin/rustpython".to_owned()); // Update this path

    interpreter.enter(|vm| {
        let scope = vm.new_scope_with_builtins();
        let source = code;
        let code_obj = vm
            .compile(source, vm::compiler::Mode::Exec, "<embedded>".to_owned())
            .map_err(|err| vm.new_syntax_error(&err, Some(source)))?;

        vm.run_code_obj(code_obj, scope)?;

        Ok(())
    })
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
