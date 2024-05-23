use anyhow::{anyhow, Result};
use reqwest;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use tokio; // Ensure you have the tokio runtime for async execution // For HTTP requests

type AsyncFn =
    Box<dyn Fn(Option<Vec<String>>) -> Pin<Box<dyn Future<Output = Result<String>>>> + Send + Sync>;
type SyncFn = Box<dyn Fn(Option<Vec<String>>) -> Result<String>>;

pub struct FunctionRegistry {
    async_fns: HashMap<String, AsyncFn>,
    sync_fns: HashMap<String, SyncFn>,
}

impl FunctionRegistry {
    pub fn new() -> Self {
        Self {
            async_fns: HashMap::new(),
            sync_fns: HashMap::new(),
        }
    }

    pub fn register_sync_function(&mut self, name: String, func: SyncFn) {
        self.sync_fns.insert(name, func);
    }

    pub fn call_sync_function(&self, name: &str, args: Option<Vec<String>>) -> Result<String> {
        self.sync_fns
            .get(name)
            .map(|f| f(args))
            .expect("failed to get sync function from registry")
    }

    pub fn register_async_function(&mut self, name: String, func: AsyncFn) {
        self.async_fns.insert(name, func);
    }

    pub async fn call_async_function(
        &self,
        name: &str,
        args: Option<Vec<String>>,
    ) -> Result<String> {
        self.async_fns
            .get(name)
            .map(|f| futures::executor::block_on(f(args)))
            .expect("failed to get async function from registry")
    }
}

pub fn get_hello_world(_args: Option<Vec<String>>) -> Result<String> {
    Ok("Hello, World!".to_string())
}

#[tokio::main]
async fn main() {
    let mut registry = FunctionRegistry::new();
    registry.register_async_function(
        "get_webpage_text".to_string(),
        Box::new(|args| Box::pin(get_webpage_text(args))),
    );

    registry.register_sync_function(
        "add_two".to_string(),
        Box::new(|args| add_two(args)),
    );

    registry.register_sync_function(
        "get_hello_world".to_string(),
        Box::new(|args| get_hello_world(args)),
    );

    match registry.call_async_function("get_webpage_text", Some(vec!["https://example.com".to_string()])).await {
        Some(Ok(text)) => println!("Webpage text: {}", text),
        _ => eprintln!("Error occurred"),
    }

    match registry.call_sync_function("add_two", Some(vec!["5".to_string(), "3".to_string()])) {
        Some(Ok(result)) => println!("Result: {}", result),
        _ => eprintln!("Error occurred"),
    }

    match registry.call_sync_function("get_hello_world", None) {
        Some(Ok(message)) => println!("{}", message),
        _ => eprintln!("Error occurred"),
    }
}