use anyhow::Result;
use autogen_rust::{immutable_agent::*, task_ledger};
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let user_proxy = ImmutableAgent::simple("user_proxy", "");

    let mut task_ledger = user_proxy
        .planning("Today is 2024-03-18. Write a blogpost about the stock price performance of Nvidia in the past month")
        .await;

    loop {
        let task = task_ledger
            .current_task()
            .clone()
            .unwrap_or("no task found".to_string())
            .clone();

        let res = user_proxy
            .next_step_by_toolcall(&task)
            .await
            .unwrap_or("no result generated".to_string());
    println!("{:?}", res.clone());

        task_ledger.record_result(res);
        if task_ledger.task_done {
            println!(
                "{:?}",
                &task_ledger
                    .result_list
                    .last()
                    .unwrap_or(&"no final result".to_string())
            );
            break;
        }
    }

    Ok(())
}
