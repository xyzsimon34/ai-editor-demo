use crate::llm::tools::extender;
use crate::llm::tools::linter;
use anyhow::Result;
use std::sync::Arc;
use yrs::Doc;

pub async fn new_composer(
    api_key: &str,
    role: &str,
    doc: &Arc<Doc>,
    user_state: &crate::editor::UserWritingState,
) -> Result<()> {
    let api_key = api_key.to_string();
    let article_draft = crate::editor::get_doc_content(doc);
    let result = extender::execute_tool(&article_draft, role, &api_key)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to execute tool: {}", e))?;
    println!("result: {}", result);

    // 使用 prepare_words 預處理單詞（添加空格和換行符）
    let words = crate::editor::prepare_words(&result);
    crate::editor::append_ai_content_word_by_word(doc, words, 100, user_state).await?;
    Ok(())
}

pub async fn new_linter(api_key: &str, doc: Arc<Doc>) -> Result<()> {
    let (_result, _updated_doc) = linter::execute_tool(doc, api_key).await?;
    Ok(())
}

pub async fn new_emoji_replacer(api_key: &str, doc: &Arc<Doc>) -> Result<()> {
    // Extract plain text from document
    let content = crate::editor::get_doc_content(doc);
    
    if content.trim().is_empty() {
        return Ok(()); // Skip if no content
    }

    // Get replacement suggestions from AI
    let replacements = crate::llm::tools::emoji_replacer::execute_tool(&content, api_key)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to execute emoji replacer tool: {}", e))?;

    if replacements.is_empty() {
        tracing::debug!("No emoji replacements suggested");
        return Ok(());
    }

    // Log what replacements we got
    tracing::info!("Received {} replacement suggestions: {:?}", replacements.len(), replacements);

    // Apply replacements to the document
    crate::editor::write::apply_replacements(doc, "content", &replacements)?;

    tracing::info!("✅ Applied {} emoji replacements", replacements.len());
    Ok(())
}
