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

    for (idx, text_ref) in text_refs.iter().enumerate() {
        let text_content = text_ref.get_string(&txn);
        let text_length = text_content.len() as u32;
        paragraph_info.push_str(&format!(
            "  TextNode[{}]: length={}, content=\"{}\"\n",
            idx, text_length, text_content
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

    let system_content = "You are a helpful writing assistant. Analyze the paragraph structure and decide where to insert new content. Use the insert_content tool to specify the insertion point and content.".to_string();

    let user_content = format!(
        "Context/Instruction: {}\n\n{}\n\nFull document content:\n{}\n\nAnalyze the paragraph and decide where to insert content using the insert_content tool.",
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
                    "description": "Insert content into a paragraph at a specific position.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "textref_index": {
                                "type": "integer",
                                "description": "The index of the text node (0-based) where to insert"
                            },
                            "offset": {
                                "type": "integer",
                                "description": "The character position within that text node (0-based)"
                            },
                            "content": {
                                "type": "string",
                                "description": "The text content to insert"
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

    // Parse the decision from function arguments
    let decision: InsertionDecision = serde_json::from_str(args_str)
        .context("Failed to parse AI insertion decision from tool call arguments")?;

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

    #[tokio::test]
    async fn test_paragraph_inserter_demo() {
        // 需要設置 OPENAI_API_KEY 環境變量
        let api_key = match env::var("OPENAI_API_KEY") {
            Ok(key) => key,
            Err(_) => {
                eprintln!("⚠️  OPENAI_API_KEY not set, skipping test");
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
        let api_key = match env::var("OPENAI_API_KEY") {
            Ok(key) => key,
            Err(_) => {
                eprintln!("⚠️  OPENAI_API_KEY not set, skipping test");
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
