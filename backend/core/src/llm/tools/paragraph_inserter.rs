use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use yrs::{Doc, GetString, Transact};

/// AI response structure for paragraph insertion
#[derive(Debug, Serialize, Deserialize)]
struct InsertionDecision {
    /// Index of the text node (textref) where to insert
    textref_index: usize,
    /// Character offset within that text node
    offset: u32,
    /// Content to insert
    content: String,
}

/// Get detailed paragraph information for AI analysis
fn get_paragraph_info(doc: &Arc<Doc>, paragraph_index: u32) -> Result<String> {
    let text_refs = crate::editor::read::get_text_refs_in_paragraph(doc, paragraph_index)
        .context("Failed to get text refs in paragraph")?;

    if text_refs.is_empty() {
        return Err(anyhow::anyhow!("Paragraph has no text nodes"));
    }

    let txn = doc.transact();
    let mut paragraph_info = String::from("Paragraph structure:\n");
    paragraph_info.push_str(&format!(
        "Total text nodes: {} (valid textref_index range: 0 to {})\n",
        text_refs.len(),
        text_refs.len().saturating_sub(1)
    ));

    for (idx, text_ref) in text_refs.iter().enumerate() {
        let text_content = text_ref.get_string(&txn);
        let text_length = text_content.len() as u32;
        paragraph_info.push_str(&format!(
            "  TextNode[{}]: length={}, content=\"{}\" (valid offset range: 0 to {})\n",
            idx, text_length, text_content, text_length
        ));
    }

    Ok(paragraph_info)
}

/// Execute the paragraph inserter tool
///
/// 1. Reads the whole paragraph structure
/// 2. Sends it to AI with context
/// 3. AI decides where to insert (textref_index, offset) and what to insert (content)
/// 4. Inserts the content at AI-decided position
pub async fn execute_tool(
    doc: &Arc<Doc>,
    paragraph_index: u32,
    context: &str,
    api_key: &str,
) -> Result<()> {
    // Step 1: Get paragraph structure information
    let paragraph_info =
        get_paragraph_info(doc, paragraph_index).context("Failed to get paragraph info")?;

    // Step 2: Get full document content for context
    let doc_content = crate::editor::read::get_doc_content(doc);

    // Step 3: Call AI to analyze paragraph and decide insertion point
    let client = reqwest::Client::new();

    let system_content = "You are a helpful writing assistant. Analyze the paragraph structure and decide where to insert new content. 

IMPORTANT: When using the insert_content tool:
- The 'content' parameter MUST be a STRING (text), not a number
- Provide the actual text content to insert, not just a number or placeholder
- Ensure all three parameters (textref_index, offset, content) are provided correctly".to_string();

    let user_content = format!(
        "Context/Instruction: {}\n\n{}\n\nFull document content:\n{}\n\nAnalyze the paragraph and decide where to insert content using the insert_content tool. Remember: the 'content' parameter must be a string containing the actual text to insert.",
        context, paragraph_info, doc_content
    );

    let request_payload = json!({
        "model": "gpt-4o-mini",
        "messages": [
            {
                "role": "system",
                "content": system_content
            },
            {
                "role": "user",
                "content": user_content
            }
        ],
        "tools": [
            {
                "type": "function",
                "function": {
                    "name": "insert_content",
                    "description": "Insert content into a paragraph at a specific position. The content parameter must be a string containing the actual text to insert.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "textref_index": {
                                "type": "integer",
                                "description": "The index of the text node (0-based) where to insert. Must be a valid integer within the range shown in the paragraph structure (typically 0 for single text node paragraphs). Check the paragraph structure to see how many text nodes exist."
                            },
                            "offset": {
                                "type": "integer",
                                "description": "The character position within that text node (0-based). Must be a valid integer."
                            },
                            "content": {
                                "type": "string",
                                "description": "The actual text content to insert. MUST be a string (text), not a number. Example: 'Hello world' or 'For example, this illustrates the point.'"
                            }
                        },
                        "required": ["textref_index", "offset", "content"]
                    }
                }
            }
        ],
        "tool_choice": {
            "type": "function",
            "function": {
                "name": "insert_content"
            }
        },
        "temperature": 0.7
    });

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&request_payload)
        .send()
        .await
        .context("Failed to connect to OpenAI during paragraph inserter execution")?;

    if !response.status().is_success() {
        let error_msg = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "Paragraph Inserter Tool Error: {}",
            error_msg
        ));
    }

    let result: serde_json::Value = response.json().await?;

    // Debug: Print the full response to understand what AI returned
    tracing::debug!(
        "OpenAI API Response: {}",
        serde_json::to_string_pretty(&result).unwrap_or_default()
    );

    // Step 4: Extract function call arguments directly from tool_calls
    let tool_calls = result["choices"][0]["message"]["tool_calls"]
        .as_array()
        .context("No tool_calls in response")?;

    // Get the first tool call (should be insert_content)
    let tool_call = tool_calls
        .first()
        .context("No tool call found in response")?;

    let function = tool_call
        .get("function")
        .context("No function in tool call")?;

    let args_str = function
        .get("arguments")
        .and_then(|v| v.as_str())
        .context("No arguments in function call")?;

    // Debug: Print the arguments string before parsing
    tracing::debug!("Function arguments string: {}", args_str);

    // Fix common AI JSON formatting errors
    // Sometimes AI returns "content=" instead of "content"
    let mut fixed_args_str = args_str.replace("\"content=\"", "\"content\"");

    // Try to parse as JSON first to check for type errors
    let parsed_value: serde_json::Value = serde_json::from_str(&fixed_args_str)
        .with_context(|| format!("Failed to parse JSON from AI response: {}", fixed_args_str))?;

    // Fix content field if it's not a string
    if let Some(content_val) = parsed_value.get("content") {
        if !content_val.is_string() {
            // If content is a number, convert it to string
            // Otherwise, try to get a string representation
            let content_str = if content_val.is_number() {
                content_val.to_string()
            } else {
                content_val
                    .as_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("{}", content_val))
            };

            // Reconstruct JSON with corrected content field
            let mut fixed_obj = parsed_value.as_object().cloned().unwrap_or_default();
            fixed_obj.insert(
                "content".to_string(),
                serde_json::Value::String(content_str),
            );
            fixed_args_str = serde_json::to_string(&fixed_obj)
                .with_context(|| "Failed to reconstruct fixed JSON")?;
        }
    }

    // Parse the decision from function arguments
    let mut decision: InsertionDecision = serde_json::from_str(&fixed_args_str)
        .with_context(|| format!("Failed to parse AI insertion decision from tool call arguments. Original: {}, Fixed: {}", args_str, fixed_args_str))?;

    // Step 4.5: Validate and fix textref_index if needed
    // Re-read text_refs to get current count (in case structure changed)
    let text_refs = crate::editor::read::get_text_refs_in_paragraph(doc, paragraph_index)
        .context("Failed to get text refs for validation")?;

    if decision.textref_index >= text_refs.len() {
        tracing::warn!(
            "AI returned textref_index {} but paragraph only has {} text nodes. Adjusting to {}",
            decision.textref_index,
            text_refs.len(),
            text_refs.len().saturating_sub(1)
        );
        // Adjust to the last valid index
        decision.textref_index = text_refs.len().saturating_sub(1);
    }

    // Step 5: Insert the AI-generated content at AI-decided position
    crate::editor::insert::insert_ai_content_to_paragraph(
        doc,
        paragraph_index,
        decision.textref_index,
        decision.offset,
        &decision.content,
    )
    .context("Failed to insert AI content into paragraph")?;

    Ok(())
}

/// Demo version with default paragraph_index = 0
pub async fn execute_tool_demo(doc: &Arc<Doc>, context: &str, api_key: &str) -> Result<()> {
    // Default: paragraph_index = 0
    execute_tool(doc, 0, context, api_key).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::read::{debug_doc_structure, get_doc_content};
    use std::env;
    use yrs::types::xml::{XmlElementPrelim, XmlOut};
    use yrs::{Transact, XmlFragment, XmlTextPrelim};

    /// 從 .env 文件加載環境變數並獲取 OPENAI_API_KEY
    ///
    /// 會嘗試從以下位置加載 .env 文件：
    /// 1. 項目根目錄的 .env
    /// 2. backend 目錄的 .env
    /// 3. 當前目錄的 .env
    fn get_api_key() -> Option<String> {
        // 嘗試從不同位置加載 .env 文件
        // dotenvy::dotenv() 會自動向上查找 .env 文件
        let _ = dotenvy::dotenv();

        // 如果 dotenv() 失敗，嘗試從常見位置加載
        if env::var("OPENAI_API_KEY").is_err() {
            let _ = dotenvy::from_filename("../.env")
                .or_else(|_| dotenvy::from_filename("../../.env"))
                .or_else(|_| dotenvy::from_filename(".env"));
        }

        env::var("OPENAI_API_KEY").ok()
    }

    #[tokio::test]
    async fn test_paragraph_inserter_demo() {
        // 從 .env 文件或環境變數中獲取 OPENAI_API_KEY
        let api_key = match get_api_key() {
            Some(key) => key,
            None => {
                eprintln!("⚠️  OPENAI_API_KEY not set, skipping test");
                eprintln!("   Please set OPENAI_API_KEY in .env file or environment variable");
                return;
            }
        };

        let doc = Arc::new(Doc::new());
        let xml_fragment = doc.get_or_insert_xml_fragment("content");

        // 創建測試段落結構
        {
            let mut txn = doc.transact_mut();
            let para = XmlElementPrelim::empty("paragraph");
            xml_fragment.insert(&mut txn, 0, para);
        }

        {
            let mut txn = doc.transact_mut();
            if let Some(XmlOut::Element(para)) = xml_fragment.get(&txn, 0) {
                para.insert(&mut txn, 0, XmlTextPrelim::new("Hello world"));
            }
        }

        // 測試前
        let before = get_doc_content(&doc);
        println!("\n=== Before Insertion ===");
        println!("Content: {}", before);
        println!("Structure:\n{}", debug_doc_structure(&doc));

        // 執行 tool
        execute_tool_demo(&doc, "Add a greeting word after 'Hello'", &api_key)
            .await
            .expect("Failed to execute paragraph inserter tool");

        // 測試後
        let after = get_doc_content(&doc);
        println!("\n=== After Insertion ===");
        println!("Content: {}", after);
        println!("Structure:\n{}", debug_doc_structure(&doc));

        // 驗證內容已改變
        assert_ne!(before, after, "Content should have changed after insertion");
        assert!(
            after.contains("Hello"),
            "Content should still contain 'Hello'"
        );
    }

    #[tokio::test]
    async fn test_paragraph_inserter_with_custom_paragraph() {
        // 從 .env 文件或環境變數中獲取 OPENAI_API_KEY
        let api_key = match get_api_key() {
            Some(key) => key,
            None => {
                eprintln!("⚠️  OPENAI_API_KEY not set, skipping test");
                eprintln!("   Please set OPENAI_API_KEY in .env file or environment variable");
                return;
            }
        };

        let doc = Arc::new(Doc::new());
        let xml_fragment = doc.get_or_insert_xml_fragment("content");

        // 創建兩個段落
        for i in 0..2 {
            let mut txn = doc.transact_mut();
            let para = XmlElementPrelim::empty("paragraph");
            xml_fragment.insert(&mut txn, i, para);
        }

        // 在第一個段落添加內容
        {
            let mut txn = doc.transact_mut();
            if let Some(XmlOut::Element(para)) = xml_fragment.get(&txn, 0) {
                para.insert(&mut txn, 0, XmlTextPrelim::new("First paragraph"));
            }
        }

        // 在第二個段落添加內容
        {
            let mut txn = doc.transact_mut();
            if let Some(XmlOut::Element(para)) = xml_fragment.get(&txn, 1) {
                para.insert(&mut txn, 0, XmlTextPrelim::new("Second paragraph"));
            }
        }

        println!("\n=== Testing Custom Paragraph (index 1) ===");
        let before = get_doc_content(&doc);
        println!("Before: {}", before);

        // 在第二個段落（index 1）插入內容
        execute_tool(
            &doc,
            1, // paragraph_index = 1 (第二個段落)
            "Add an example sentence",
            &api_key,
        )
        .await
        .expect("Failed to execute paragraph inserter tool");

        let after = get_doc_content(&doc);
        println!("After: {}", after);
        println!("Structure:\n{}", debug_doc_structure(&doc));

        assert_ne!(before, after);
    }
}
