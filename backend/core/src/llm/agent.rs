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
