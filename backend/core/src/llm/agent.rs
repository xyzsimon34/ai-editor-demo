use crate::llm::tools::extender;
use crate::llm::tools::linter;
use anyhow::Result;
use std::sync::Arc;
use yrs::Doc;

pub async fn new_composer(api_key: &str, role: &str, doc: &Arc<Doc>) -> Result<()> {
    let api_key = api_key.to_string();
    let article_draft = crate::editor::get_doc_content(doc);
    let result = extender::execute_tool(&article_draft, role, &api_key)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to execute tool: {}", e))?;
    println!("result: {}", result);
    let user_state = crate::editor::UserWritingState::new(3);
    // let words = result.char_indices().map(|(_, c)| c.to_string()).collect();
    let words = result.split_whitespace().map(|w| w.to_string()).collect();
    crate::editor::append_ai_content_word_by_word(doc, words, 10, &user_state).await?;
    Ok(())
}

pub async fn new_linter(api_key: &str, doc: Arc<Doc>) -> Result<()> {
    let (_result, _updated_doc) = linter::execute_tool(doc, api_key).await?;
    Ok(())
}
