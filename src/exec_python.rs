use crate::immutable_agent::{get_user_feedback, save_py_to_disk};
use crate::llm_utils::chat_inner_async_wrapper_text;
use crate::{QWEN_CONFIG, RUN_FUNC_REACT, TOGETHER_CONFIG};
use anyhow::Result;
use regex::Regex;
use std::io::{BufRead, BufReader};
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use tokio::sync::mpsc;
use tokio::task;

pub async fn run_python_wrapper(code_wrapped_in_text: &str) -> (bool, String, String) {
    println!("raw code: {:?}\n\n", code_wrapped_in_text);
    let code = extract_code(code_wrapped_in_text);
    println!("clean code: {:?}\n\n", code.clone());

    let _ = save_py_to_disk("src/test.py", &code).await;

    match run_python_func_react("/Users/jichen/Projects/autogen_rust/src/test.py").await {
        Ok(success_result_text) => {
            println!("success: {:?}", success_result_text);

            (true, code, success_result_text)
        }
        Err(err_msg) => {
            println!("failure: {:?}", err_msg.to_string());

            (false, code, err_msg.to_string())
        }
    }
}

pub async fn llm_play(input: &str) -> Result<String> {
    let res = chat_inner_async_wrapper_text(&QWEN_CONFIG, &RUN_FUNC_REACT, input, 1).await?;

    Ok(res)
}

pub async fn run_python_func_test(func_path: &str) -> Result<String> {
    let mut cmd = Command::new("/Users/jichen/miniconda3/bin/python")
        .arg(func_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut stdout = cmd.stdout.take().unwrap();
    let mut stderr = cmd.stderr.take().unwrap();
    let mut stdin = cmd.stdin.take().unwrap();

    let (stdout_tx, mut stdout_rx) = mpsc::channel(100);

    let stdout_task = task::spawn(async move {
        let mut stdout_output = String::new();
        let mut buffer = [0; 1024];
        loop {
            match stdout.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    let output = String::from_utf8_lossy(&buffer[0..n]).to_string();
                    stdout_output.push_str(&output);
                    stdout_tx.send(output).await?;
                }
                Err(e) => return Err(anyhow::anyhow!("Failed to read from stdout: {}", e)),
            }
        }
        Ok(stdout_output)
    });

    let mut combined_output = String::new();

    println!("before await:");
    let status = cmd.wait()?;
    println!("status: {:?}", status.clone());
    // why there is no status.err() branch? what kind code are you expecting in this arm?
    match (status.success(), status.code()) {
        (true, _) => {
            while let Some(line) = stdout_rx.recv().await {
                let input = line.trim().to_string();
                combined_output.push_str(&input);
                combined_output.push('\n');

                pretty_print_board(&input);
            }
            Ok(combined_output)
        }
        // further branch it 2 arms, then handle accordingly
        (false, _) => {
            //   (false, Some(code)) | (false, None) => {
            while let Some(line) = stdout_rx.recv().await {
                let input = line.trim().to_string();
                combined_output.push_str(&input);
                combined_output.push('\n');

                pretty_print_board(&input);
                match get_user_feedback().await {
                    Ok(human_input) => {
                        combined_output.push_str(&human_input);
                        combined_output.push('\n');
                        stdin.write_all(human_input.as_bytes()).unwrap();
                        stdin.write_all(b"\n").unwrap();
                    }
                    Err(_) => break,
                };
            }

            let mut stderr_output = String::new();
            let _ = stderr.read_to_string(&mut stderr_output)?;

            let stdout_result = stdout_task.await??;

            let final_error = format!("{}\n{}", combined_output, stderr_output);
            let error_message = match status.code() {
                Some(code) => format!("Error: Process exited with code {}: {}", code, final_error),
                None => format!("Error: Process terminated by signal: {}", final_error),
            };
            Err(anyhow::anyhow!(error_message))
        }
    }
}
pub async fn run_python_func_react(func_path: &str) -> Result<String> {
    //what if I don't pipe stderr, I just get its value at the end? explain it
    let mut cmd = Command::new("/Users/jichen/miniconda3/bin/python")
        .arg(func_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut stdin = cmd.stdin.take().unwrap();
    let mut stdout = cmd.stdout.take().unwrap();

    let (stdout_tx, mut stdout_rx) = mpsc::channel(100);
    //need to check somewhere that the python code has successfully concluded, no need to get human input
    let stdout_task = task::spawn(async move {
        let mut buffer = [0; 1024];
        loop {
            match stdout.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    let output = String::from_utf8_lossy(&buffer[0..n]).to_string();
                    let _ = stdout_tx.send(output).await;
                }
                _ => (),
            }
        }
    });

    let mut combined_output = String::new();

    if let Ok(status) = cmd.wait() {
        println!("Command finished with status: {:?}", status);
        if status.code() == Some(0) {
            while let Some(block) = stdout_rx.recv().await {
                let input = block.trim().to_string();
                combined_output.push_str(&input);
            }
        }
    } else {
        println!("Failed to check command status");
        while let Some(block) = stdout_rx.recv().await {
            let input = block.trim().to_string();
            combined_output.push_str(&input);

            pretty_print_board(&input);
            // this block is not executed
            /* match cmd.wait() {
                Err(_) => break,
                Ok(res) if res.success() => break,
                Ok(res) => match get_user_feedback().await {
                    Ok(human_input) => {
                        combined_output.push_str(&human_input);
                        combined_output.push('\n');
                        stdin.write_all(human_input.as_bytes()).unwrap();
                        stdin.write_all(b"\n").unwrap();
                    }
                    Err(_) => break,
                },
            }; */
            if let Ok(human_input) = get_user_feedback().await {
                combined_output.push_str(&human_input);
                combined_output.push('\n');
                stdin.write_all(human_input.as_bytes()).unwrap();
                stdin.write_all(b"\n").unwrap();
            }
        }
    }

    // is this line really needed?
    let _ = stdout_task.await;

    let status = cmd.wait()?;

    if status.success() {
        Ok(combined_output)
    } else {
        let mut stderr_output = String::new();
        if let Some(mut stderr) = cmd.stderr.take() {
            let _ = stderr.read_to_string(&mut stderr_output)?;
        }
        let final_error = format!("{}\n{}", combined_output, stderr_output);
        Err(anyhow::anyhow!("Error: {}", final_error))
    }
}

fn pretty_print_board(board: &str) {
    let lines: Vec<&str> = board.split('\n').collect();
    for line in lines {
        println!("{}", line);
    }
}
pub async fn run_python_func(func_path: &str) -> Result<String> {
    let mut cmd = Command::new("/Users/jichen/miniconda3/bin/python")
        .arg(func_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = cmd.stdout.take().unwrap();
    let mut stderr = cmd.stderr.take().unwrap();
    let mut stderr_output = String::new();
    let mut stdout_output = String::new();

    let mut stdout_lines = BufReader::new(stdout).lines();
    let _ = stderr.read_to_string(&mut stderr_output)?;

    while let Some(line) = stdout_lines.next() {
        stdout_output.push_str(&line?);
        stdout_output.push('\n');
    }

    let status = cmd.wait()?;

    if status.success() {
        Ok(stdout_output)
    } else {
        Err(anyhow::anyhow!("Error: {}", stderr_output))
    }
}

pub fn extract_code(text: &str) -> String {
    let multi_line_pattern = r"(?s)```python(.*?)```";
    let mut program = String::new();

    let multi_line_regex = Regex::new(multi_line_pattern).unwrap();
    for cap in multi_line_regex.captures_iter(text) {
        if let Some(code) = cap.get(1) {
            program.push_str(code.as_str().trim());
        }
    }

    program
}

pub fn extract_code_blocks(
    text: &str,
    detect_single_line_code: bool,
) -> Vec<(Option<String>, String)> {
    // Adjust regex pattern to handle both Unix and Windows line endings and optional language specifier
    let multi_line_pattern = r"```[ \t]*(\w+)?[ \t]*\r?\n(.*?)\r?\n[ \t]*```";
    let single_line_pattern = r"`([^`]+)`";
    let mut results: Vec<(Option<String>, String)> = Vec::new();

    let multi_line_regex = Regex::new(multi_line_pattern).unwrap();
    for cap in multi_line_regex.captures_iter(text) {
        let language = cap
            .get(1)
            .map_or(None, |m| Some(m.as_str().trim().to_string()));
        let code = cap.get(2).unwrap().as_str().trim().to_string();
        results.push((language.clone(), code.clone()));
        // println!("Matched multi-line code block: Language: {:?}, Code: {}", language, code);
    }

    if detect_single_line_code {
        let single_line_regex = Regex::new(single_line_pattern).unwrap();
        for cap in single_line_regex.captures_iter(text) {
            results.push((None, cap.get(1).unwrap().as_str().trim().to_string()));
            // println!("Matched single-line code: {}", cap.get(1).unwrap().as_str().trim());
        }
    }

    results
}

// export DYLD_LIBRARY_PATH=/Users/jichen/miniconda3/lib:$DYLD_LIBRARY_PATH
// export PYO3_PYTHON=/Users/jichen/miniconda3/bin/python
// export DYLD_LIBRARY_PATH=/Users/jichen/miniconda3/lib
