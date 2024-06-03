use anyhow::Result;
use autogen_rust::immutable_agent::*;
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let user_proxy = ImmutableAgent::simple("user_proxy", "");

    let task_vec = user_proxy.planning("find when John Lennon was assasinnated").await;

    let res = user_proxy.stepper(&task_vec).await;
    println!("{:?}", res);

    Ok(())
}
