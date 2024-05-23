use anyhow::{anyhow, Result};
use regex::Regex;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

#[macro_export]
macro_rules! call_function_async {
    ($func:expr) => {
        async { $func().map_err(|e| e.to_string()) }
    };

    ($func:expr, $args:expr, single) => {
        async {
            let re = Regex::new(r#""[^"]+":\s*"([^"]+)""#).expect("Failed to parse arguments");
            let arg = re
                .captures_iter($args)
                .filter_map(|cap| cap.get(1).map(|m| m.as_str()))
                .next()
                .expect("No argument found")
                .to_string();

            $func(arg).map_err(|e| e.to_string())
        }
    };

    ($func:expr, $args:expr, multi) => {
        async {
            let re = Regex::new(r#""[^"]+":\s*"([^"]+)""#)
                .unwrap_or_else(|_| panic!("Failed to parse arguments"));
            let args: Vec<&str> = re
                .captures_iter($args)
                .filter_map(|cap| cap.get(1).map(|m| m.as_str()))
                .collect();

            $func(args).map_err(|e| e.to_string())
        }
    };
}

#[macro_export]
macro_rules! call_function {
    ($func:expr) => {{
        $func()
    }};

    ($func:expr, $args:expr, single) => {{
        let re = Regex::new(r#""[^"]+":\s*"([^"]+)""#)
            .unwrap_or_else(|_| panic!("Failed to parse arguments"));
        let arg = re
            .captures_iter($args)
            .filter_map(|cap| cap.get(1).map(|m| m.as_str()))
            .next()
            .ok_or_else(|| anyhow!("No argument found"))?;

        $func(arg)
    }};

    ($func:expr, $args:expr, multi) => {{
        let re = Regex::new(r#""[^"]+":\s*"([^"]+)""#)
            .unwrap_or_else(|_| panic!("Failed to parse arguments"));
        let args: Vec<String> = re
            .captures_iter($args)
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
            .collect();

        $func(&args)
    }};
}

type SyncFn = fn(String) -> Result<String>; // Function takes a single String argument
type AsyncFn =
    Box<dyn Fn(Vec<String>) -> Pin<Box<dyn Future<Output = Result<String>>>> + Send + Sync>; 

    pub struct FunctionRegistry {
    pub sync_functions: HashMap<String, SyncFn>,
    pub async_functions: HashMap<String, AsyncFn>,
}

impl FunctionRegistry {
    pub fn new() -> Self {
        FunctionRegistry {
            sync_functions: HashMap::new(),
            async_functions: HashMap::new(),
        }
    }

    pub fn register_sync_function(&mut self, name: &str, func: SyncFn) {
        self.sync_functions.insert(name.to_string(), func);
    }

    pub fn register_async_function(&mut self, name: &str, func: AsyncFn) {
        self.async_functions.insert(name.to_string(), func);
    }

    pub fn call_sync(&self, name: &str, args: &str) -> Result<String> {
        let colon_count = args.chars().filter(|&c| c == ':').count();
        match self.sync_functions.get(name) {
            Some(&func) => match colon_count {
                0 => call_function!(func),
                1 => call_function!(func, args, single),
                _ => call_function!(func, args, multi),
            },
            None => Err(anyhow!("Function not found")),
        }
    }

    pub async fn call_async(&self, name: &str, args: &str) -> anyhow::Result<String, String> {
        let colon_count = args.chars().filter(|&c| c == ':').count();
        match self.sync_functions.get(name) {
            Some(&func) => match colon_count {
                0 => call_function_async!(func).await,
                1 => call_function_async!(func, args, single).await,
                _ => call_function_async!(func, args, multi).await,
            },
            None => Err("Function not found".to_string()),
        }
    }
}
