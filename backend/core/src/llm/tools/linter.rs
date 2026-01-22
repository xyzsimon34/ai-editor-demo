use crate::editor::write::{clear_element_content, get_element};
use anyhow::{Context, Result};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, debug_span, instrument};
use yrs::types::xml::XmlFragmentRef;
use yrs::{Doc, GetString, Text, Transact, Xml, XmlFragment};

fn xml_fragment_to_string(doc: &Doc, fragment: &XmlFragmentRef) -> String {
    let txn = doc.transact();
    let mut result = String::new();
    let len = fragment.len(&txn);
    for i in 0..len {
        if let Some(child) = fragment.get(&txn, i) {
            result.push_str(&xml_node_to_string(&child, &txn));
        }
    }
    result
}

fn xml_node_to_string(node: &yrs::types::xml::XmlOut, txn: &impl yrs::ReadTxn) -> String {
    match node {
        yrs::types::xml::XmlOut::Element(elem) => {
            let mut result = String::new();
            result.push('<');
            result.push_str(elem.tag().as_ref());

            // Add attributes
            let attrs = elem.attributes(txn);
            for (key, value) in attrs {
                result.push(' ');
                result.push_str(key.as_ref());
                result.push_str("=\"");
                result.push_str(&value.to_string(txn));
                result.push('"');
            }

            result.push('>');

            // Add children
            let len = elem.len(txn);
            for i in 0..len {
                if let Some(child) = elem.get(txn, i) {
                    result.push_str(&xml_node_to_string(&child, txn));
                }
            }

            result.push_str("</");
            result.push_str(elem.tag().as_ref());
            result.push('>');
            result
        }
        yrs::types::xml::XmlOut::Text(text) => text.get_string(txn),
        yrs::types::xml::XmlOut::Fragment(_) => String::new(),
    }
}

#[instrument(
    skip(doc, api_key), // 隱藏大物件或敏感資訊
    fields(run_id = tracing::field::Empty) // 先挖個洞，稍後填入
)]
pub async fn execute_tool(doc: Arc<Doc>, api_key: &str) -> Result<()> {
    let run_id = format!(
        "linter-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis()
    );

    tracing::Span::current().record("run_id", &run_id);

    let fragment = doc.get_or_insert_xml_fragment("content");

    let original_xml = xml_fragment_to_string(&doc, &fragment);
    let original_xml = original_xml
        .replace("<paragraph>", "")
        .replace("</paragraph>", "");

    let client = reqwest::Client::new();

    let system_content = r#"You are the "Linguistic Diff Engine." Your task is to perform a character-level or word-level diff to correct grammar and spelling while preserving the exact structure of the original text.

    ### Operational Rules:
    1. **Markup Logic**: 
        - Use <del>text</del> for the exact characters/words being removed.
        - Use <ins>text</ins> for the exact characters/words being added.
        - **Do not** add new punctuation or change casing unless it is explicitly part of the correction within the tags.
        - All unchanged text must remain exactly as it was.
    2. **Linguistic Scope**: Fix grammar, spelling, punctuation, and capitalization.
    3. **Output Constraint**: 
        - Return ONLY the processed text string.
        - Do NOT include any explanations, markdown blocks, or extra formatting.

    ### Example Input:
    Helo world.

    ### Example Output:
    <del>Helo</del><ins>Hello</ins> world."#;

    let user_content = format!("{}", original_xml);

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
        ]
    });

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&request_payload)
        .send()
        .await
        .context("Failed to connect to OpenAI during linter execution")?;

    if !response.status().is_success() {
        let error_msg = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("Linter Tool Error: {}", error_msg));
    }

    let result: serde_json::Value = response.json().await?;

    let ai_output = result["choices"][0]["message"]["content"]
        .as_str()
        .context("Failed to get content from Linter response")?
        .to_string();

    {
        let _span = debug_span!("xml_update", run_id = %run_id).entered();
        debug!("Replacing XML fragment content");

        let mut mark_attrs = std::collections::HashMap::<String, yrs::Any>::new();
        mark_attrs.insert("status".to_string(), yrs::Any::String("pending".into()));
        mark_attrs.insert("aimodel".to_string(), yrs::Any::String("gpt-4".into()));
        mark_attrs.insert("tool".to_string(), yrs::Any::String("linter".into()));
        mark_attrs.insert("runid".to_string(), yrs::Any::String(run_id.clone().into()));
        mark_attrs.insert("model".to_string(), yrs::Any::String("gpt-4o-mini".into()));

        parse_xml_to_element(&doc, &mut mark_attrs, &ai_output)?;
    }

    Ok(())
}

fn parse_xml_to_element(
    doc: &Doc,
    attributes: &mut HashMap<String, yrs::Any>,
    xml: &str,
) -> Result<Vec<String>> {
    let mut result = Vec::new();
    let mut chars = xml.chars().peekable();
    let mut current_text = String::new();
    
    let elem = get_element(doc, "content")?;
    let mut txn = doc.transact_mut();
    
    // Clear element content only once at the beginning
    clear_element_content(&elem, &mut txn)?;
    
    let mut insert_pos = 0;

    while chars.peek().is_some() {
        if *chars.peek().unwrap() == '<' {
            // Process any accumulated text before the tag
            if !current_text.is_empty() {
                let text_node = elem.insert(&mut txn, insert_pos, yrs::XmlTextPrelim::new(""));

                let mut text_attrs = std::collections::HashMap::<Arc<str>, yrs::Any>::new();
                text_attrs.insert(
                    Arc::from("aisuggestion"),
                    yrs::Any::Map(Arc::new(attributes.clone())),
                );

                tracing::info!("content: {}", current_text);
                tracing::info!("text_attrs: {:?}", text_attrs);
                text_node.insert_with_attributes(&mut txn, 0, &current_text, text_attrs);
                insert_pos += 1;
                current_text.clear();
            }

            let (content, updated_attrs) = parse_tag_as_text(&mut chars, attributes.clone());

            // Only create text node if content is not empty
            if !content.is_empty() {
                let text_node = elem.insert(&mut txn, insert_pos, yrs::XmlTextPrelim::new(""));

                let mut text_attrs = std::collections::HashMap::<Arc<str>, yrs::Any>::new();
                text_attrs.insert(
                    Arc::from("aisuggestion"),
                    yrs::Any::Map(Arc::new(updated_attrs)),
                );

                tracing::info!("content: {}", content);
                tracing::info!("text_attrs: {:?}", text_attrs);
                text_node.insert_with_attributes(&mut txn, 0, &content, text_attrs);
                insert_pos += 1;
            }
        } else {
            current_text.push(chars.next().unwrap());
        }
    }
    // Process any remaining text after all tags
    if !current_text.is_empty() {
        let text_node = elem.insert(&mut txn, insert_pos, yrs::XmlTextPrelim::new(""));

        let mut text_attrs = std::collections::HashMap::<Arc<str>, yrs::Any>::new();
        text_attrs.insert(
            Arc::from("aisuggestion"),
            yrs::Any::Map(Arc::new(attributes.clone())),
        );

        tracing::info!("content: {}", current_text);
        tracing::info!("text_attrs: {:?}", text_attrs);
        text_node.insert_with_attributes(&mut txn, 0, &current_text, text_attrs);
    }

    Ok(result)
}

fn parse_tag_as_text(
    chars: &mut std::iter::Peekable<std::str::Chars>,
    attributes: HashMap<String, yrs::Any>,
) -> (String, HashMap<String, yrs::Any>) {
    let mut attributes = attributes.clone();

    // Parse opening tag (e.g., <del> or <del class="x">)
    let mut opening_tag = String::new();
    while chars.peek().is_some() {
        if *chars.peek().unwrap() == '>' {
            opening_tag.push(chars.next().unwrap());
            break;
        }
        opening_tag.push(chars.next().unwrap());
    }

    // Check if we have more characters after the opening tag
    if chars.peek().is_none() {
        return (opening_tag, attributes);
    }

    // Determine expected closing tag based on opening tag
    let expected_closing_tag = if opening_tag == "<del>" {
        attributes.insert("operation".to_string(), yrs::Any::String("delete".into()));
        "</del>"
    } else if opening_tag == "<ins>" {
        attributes.insert("operation".to_string(), yrs::Any::String("insert".into()));
        "</ins>"
    } else {
        // Not a del/ins tag, collect all remaining content and return
        let mut all_content = opening_tag;
        while chars.peek().is_some() {
            all_content.push(chars.next().unwrap());
        }
        return (all_content, attributes);
    };

    // Collect content between opening and closing tags
    let mut buffer = String::new();
    let closing_tag_len = expected_closing_tag.len();

    while chars.peek().is_some() {
        let ch = chars.next().unwrap();
        buffer.push(ch);

        // Check if buffer ends with the closing tag
        if buffer.len() >= closing_tag_len && buffer.ends_with(expected_closing_tag) {
            // Found matching closing tag, remove it and return content
            buffer.truncate(buffer.len() - closing_tag_len);
            return (buffer, attributes);
        }
    }

    // No matching closing tag found, return everything consumed
    (buffer, attributes)
}
