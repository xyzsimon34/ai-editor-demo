use crate::llm::tools::extender;

pub async fn new_agent(api_key: &str, role: &str) -> Result<String, anyhow::Error> {
    let api_key = api_key.to_string();

    let plain_text =
        "In Taipei in 2050, AI will no longer be merely a tool, but a conscious citizen. This";

    let result = extender::execute_tool(plain_text, role, &api_key)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to execute tool: {}", e))?;
    println!("result: {}", result);
    write_to_yjs(&result);
    Ok(result.clone())
}

pub fn write_to_yjs(plain_text: &str) {
    print!("DONE");
}
