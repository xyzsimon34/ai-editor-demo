use crate::llm::tools::extender;
use crate::llm::tools::linter;
use anyhow::Result;
use std::sync::{Arc, atomic::AtomicU64};
use yrs::{Doc, Transact, XmlFragment};
pub async fn new_composer(
    api_key: &str,
    role: &str,
    doc: &Arc<Doc>,
    user_last_used_at: Arc<AtomicU64>,
    user_writing_timeout_ms: u64,
    preview_mode: bool,
) -> Result<Option<String>> {
    let api_key = api_key.to_string();
    let article_draft = crate::editor::get_doc_content(doc);
    let result = extender::execute_tool(&article_draft, role, &api_key)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to execute tool: {}", e))?;
    println!("result: {}", result);
    // THIS IS THE MARKS FLOW. USE IF ELEMENt FLOW DOESN'T WORK
    // 使用 prepare_words 預處理單詞（添加空格和換行符）
    // let words = crate::editor::format_word_stream(&result);
    // crate::editor::append_ai_content_word_by_word(doc, words, 100, user_state).await?;
    // Generate a unique run ID for this AI generation
    let run_id = format!(
        "extender-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );
    crate::editor::append_ai_content_to_doc(doc, &result, Some("extender"), Some(&run_id))?;

    if preview_mode {
        return Ok(Some(result));
    }

    // 使用 format_word_stream 預處理單詞（添加空格和換行符）
    // let words = crate::editor::format_word_stream(&result);
    // crate::editor::append_ai_content_word_by_word(doc, words, 100, user_state).await?;
    // let fragment = doc.get_or_insert_xml_fragment("content");
    // let mut txn = doc.transact_mut();

    // if let Some(root_elem) = fragment.get(&txn, 0) {
    //     crate::editor::push_element(
    //         &root_elem,
    //         "ai_generated",
    //         &result,
    //         &[("data-ai-generated", "true"), ("data-ai-id", "ai-345")],
    //         &mut txn,
    //     )?;
    // } else {
    //     tracing::warn!("⚠️ Fragment has no root element to append AI content to");
    // }

    Ok(None)
}

pub async fn new_linter(api_key: &str, doc: Arc<Doc>) -> Result<()> {
    let _ = linter::execute_tool(doc, api_key).await?;
    Ok(())
}

pub async fn new_backseating_agent(
    api_key: &str,
    doc: &Arc<Doc>,
) -> Result<Vec<crate::llm::tools::backseater::BackseaterArgs>> {
    let content = crate::editor::get_doc_content(doc);
    if content.trim().is_empty() {
        tracing::info!("⚠️ Content is empty, skipping backseating agent");
        return Ok(Vec::new());
    }

    tracing::info!("🔄 Calling OpenAI API for backseater comments (direct function calling)...");
    // Use direct function calling - single API call, extract tool call arguments directly
    // No Agent loop needed since tool arguments ARE the final answer
    let comments = crate::llm::tools::backseater::execute_tool(&content, api_key)
        .await
        .map_err(|e| {
            tracing::error!("❌ Failed to execute backseater tool: {:?}", e);
            anyhow::anyhow!("Failed to execute backseater tool: {}", e)
        })?;

    tracing::info!("📝 Generated {} comments from backseater", comments.len());
    Ok(comments)
}

pub async fn new_emoji_replacer(api_key: &str, doc: &Arc<Doc>) -> Result<()> {
    // Extract plain text from document
    let content = crate::editor::get_doc_content(doc);
    if content.trim().is_empty() {
        tracing::info!("⚠️ Content is empty, skipping emoji replacer");
        return Ok(()); // Skip if no content
    }
    // Get replacement suggestions from AI
    let replacements = crate::llm::tools::emoji_replacer::execute_tool(&content, api_key)
        .await
        .map_err(|e| {
            tracing::error!("❌ Failed to execute emoji replacer tool: {:?}", e);
            anyhow::anyhow!("Failed to execute emoji replacer tool: {}", e)
        })?;

    if replacements.is_empty() {
        tracing::info!("⚠️ No emoji replacements suggested by AI, skipping");
        return Ok(());
    }

    // Apply replacements to the document
    crate::editor::write::apply_replacements(doc, "content", &replacements).map_err(|e| {
        tracing::error!("❌ Failed to apply replacements: {:?}", e);
        e
    })?;

    tracing::info!(
        "✅ Successfully applied {} emoji replacements",
        replacements.len()
    );
    Ok(())
}
