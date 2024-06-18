use core::task;

use anyhow::Result;
use autogen_rust::{immutable_agent::*, task_ledger};
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let user_proxy = ImmutableAgent::simple("user_proxy", "");

    let (mut task_ledger, solution) = user_proxy
        // .planning("tell me a joke")
        .planning("find about how old is Joe Biden")
        // .planning("Today is 2024-03-18. Write a blogpost about the stock price performance of Nvidia in the past month")
        .await;

    // if task_ledger.task_list.is_empty() && solution.is_some() {
    //     println!("solution: {:?} ", solution);
    //     std::process::exit(0);
    // }

    loop {
        let task_summary = task_ledger.clone().parent_task.unwrap_or("TERMINATE".to_string()).clone();
        let task = task_ledger
            .current_task()
            .unwrap_or(task_summary);

        let carry_over = match task_ledger.solution_list.last() {
            Some(c) => Some(c.to_string()),
            None => None,
        };

        let res = user_proxy
            .next_step_by_toolcall_nested(carry_over, &task)
            .await
            .unwrap_or("no result generated".to_string());
        // println!("{:?}", res.clone());


        // tokio::time::sleep(std::time::Duration::from_secs(20)).await;

        if !task_ledger.record_solution(res) {
            break;
        }
    }

    println!(
        "{:?}",
        &task_ledger
            .solution_list
            .last()
            .unwrap_or(&"no final result".to_string())
    );

    Ok(())
}
