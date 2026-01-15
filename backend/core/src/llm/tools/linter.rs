use anyhow::{Context, Result};
use serde_json::json;
use std::sync::Arc;
use tracing::info;
use yrs::types::xml::{XmlElementRef, XmlFragmentRef};
use yrs::{Doc, GetString, Transact, Xml, XmlFragment};

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

fn replace_xml_fragment_content(doc: &Doc, fragment: &XmlFragmentRef, new_xml: &str) -> Result<()> {
    let mut txn = doc.transact_mut();

    // Clear existing content
    let len = fragment.len(&txn);
    if len > 0 {
        fragment.remove_range(&mut txn, 0, len);
    }

    // Parse and insert new XML
    // For simplicity, we'll use a basic XML parser approach
    // In production, you'd want to use a proper XML parser
    let parsed = parse_xml_string(new_xml)?;
    insert_xml_prelim(&mut txn, fragment, &parsed);

    Ok(())
}

#[derive(Debug, Clone)]
enum XmlPrelim {
    Element {
        tag: String,
        attrs: Vec<(String, String)>,
        children: Vec<XmlPrelim>,
    },
    Text(String),
}

fn parse_xml_string(xml: &str) -> Result<Vec<XmlPrelim>> {
    // Simple XML parser - handles basic cases
    // This is a simplified version, for production use a proper XML parser
    let mut result = Vec::new();
    let mut chars = xml.chars().peekable();

    while chars.peek().is_some() {
        skip_whitespace(&mut chars);
        if chars.peek().is_none() {
            break;
        }

        if *chars.peek().unwrap() == '<' {
            let elem = parse_element(&mut chars)?;
            result.push(elem);
        } else {
            let text = parse_text(&mut chars);
            if !text.trim().is_empty() {
                result.push(XmlPrelim::Text(text));
            }
        }
    }

    Ok(result)
}

fn skip_whitespace(chars: &mut std::iter::Peekable<std::str::Chars>) {
    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
        } else {
            break;
        }
    }
}

fn parse_element(chars: &mut std::iter::Peekable<std::str::Chars>) -> Result<XmlPrelim> {
    assert_eq!(chars.next(), Some('<'));

    let tag = parse_tag_name(chars)?;
    let mut attrs = Vec::new();

    skip_whitespace(chars);

    // Parse attributes
    while chars.peek().map(|&c| c != '>' && c != '/') == Some(true) {
        skip_whitespace(chars);
        if chars.peek().map(|&c| c == '>' || c == '/') == Some(true) {
            break;
        }
        let key = parse_attr_name(chars)?;
        skip_whitespace(chars);
        if chars.peek() == Some(&'=') {
            chars.next();
            skip_whitespace(chars);
            let value = parse_attr_value(chars)?;
            attrs.push((key, value));
        }
        skip_whitespace(chars);
    }

    if chars.peek() == Some(&'/') {
        // Self-closing tag
        chars.next();
        assert_eq!(chars.next(), Some('>'));
        return Ok(XmlPrelim::Element {
            tag,
            attrs,
            children: Vec::new(),
        });
    }

    assert_eq!(chars.next(), Some('>'));

    // Parse children
    let mut children = Vec::new();
    loop {
        skip_whitespace(chars);
        if chars.peek().is_none() {
            break;
        }
        if chars.peek() == Some(&'<') {
            let next_char = chars.clone().nth(1);
            if next_char == Some('/') {
                // Closing tag
                break;
            }
            let child = parse_element(chars)?;
            children.push(child);
        } else {
            let text = parse_text(chars);
            if !text.trim().is_empty() {
                children.push(XmlPrelim::Text(text));
            }
        }
    }

    // Parse closing tag
    assert_eq!(chars.next(), Some('<'));
    assert_eq!(chars.next(), Some('/'));
    let closing_tag = parse_tag_name(chars)?;
    assert_eq!(closing_tag, tag);
    assert_eq!(chars.next(), Some('>'));

    Ok(XmlPrelim::Element {
        tag,
        attrs,
        children,
    })
}

fn parse_tag_name(chars: &mut std::iter::Peekable<std::str::Chars>) -> Result<String> {
    let mut name = String::new();
    while let Some(&ch) = chars.peek() {
        if ch.is_alphanumeric() || ch == '-' || ch == '_' {
            name.push(chars.next().unwrap());
        } else {
            break;
        }
    }
    if name.is_empty() {
        return Err(anyhow::anyhow!("Empty tag name"));
    }
    Ok(name)
}

fn parse_attr_name(chars: &mut std::iter::Peekable<std::str::Chars>) -> Result<String> {
    parse_tag_name(chars)
}

fn parse_attr_value(chars: &mut std::iter::Peekable<std::str::Chars>) -> Result<String> {
    if chars.peek() != Some(&'"') {
        return Err(anyhow::anyhow!("Expected quoted attribute value"));
    }
    chars.next();
    let mut value = String::new();
    while let Some(ch) = chars.next() {
        if ch == '"' {
            break;
        }
        value.push(ch);
    }
    Ok(value)
}

fn parse_text(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut text = String::new();
    while let Some(&ch) = chars.peek() {
        if ch == '<' {
            break;
        }
        text.push(chars.next().unwrap());
    }
    text
}

fn insert_xml_prelim(
    txn: &mut yrs::TransactionMut,
    fragment: &XmlFragmentRef,
    prelims: &[XmlPrelim],
) {
    for prelim in prelims {
        match prelim {
            XmlPrelim::Element {
                tag,
                attrs,
                children,
            } => {
                let elem_prelim = yrs::types::xml::XmlElementPrelim::empty(tag.as_str());
                let elem = fragment.insert(txn, fragment.len(txn), elem_prelim);

                for (key, value) in attrs {
                    elem.insert_attribute(txn, key.as_str(), value.as_str());
                }

                for child in children {
                    insert_xml_prelim_into_element(txn, &elem, child);
                }
            }
            XmlPrelim::Text(text) => {
                fragment.insert(txn, fragment.len(txn), yrs::XmlTextPrelim::new(text));
            }
        }
    }
}

fn insert_xml_prelim_into_element(
    txn: &mut yrs::TransactionMut,
    elem: &XmlElementRef,
    prelim: &XmlPrelim,
) {
    match prelim {
        XmlPrelim::Element {
            tag,
            attrs,
            children,
        } => {
            let elem_prelim = yrs::types::xml::XmlElementPrelim::empty(tag.as_str());
            let child_elem = elem.insert(txn, elem.len(txn), elem_prelim);

            for (key, value) in attrs {
                child_elem.insert_attribute(txn, key.as_str(), value.as_str());
            }

            for child in children {
                insert_xml_prelim_into_element(txn, &child_elem, child);
            }
        }
        XmlPrelim::Text(text) => {
            elem.insert(txn, elem.len(txn), yrs::XmlTextPrelim::new(text));
        }
    }
}

pub async fn execute_tool(doc: Arc<Doc>, api_key: &str) -> Result<(String, Arc<Doc>)> {
    let fragment = doc.get_or_insert_xml_fragment("content");

    // Get original XML string
    let original_xml = xml_fragment_to_string(&doc, &fragment);

    let client = reqwest::Client::new();

    let system_content = r#"You are the "Schema Sentry," a specialized linguistic linter for Yjs XmlFragments.

Your sole purpose is to:
1. Fix grammatical errors and spelling mistakes within the text nodes.
2. Refine vocabulary for better clarity while maintaining the original tone.
3. Strict Constraint: Do NOT provide any explanations, comments, or markdown code blocks (like ```xml).
4. Output Format: Return ONLY the complete, corrected XML string. Do NOT change the XML tag names or structure; only improve the text content within them.
5. If no errors are found, return the original XML string exactly as it is."#;

    let request_payload = json!({
        "model": "gpt-4o-mini",
        "messages": [
            {
                "role": "system",
                "content": system_content
            },
            {
                "role": "user",
                "content": original_xml
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

    // Replace content with AI output
    info!("Linter response: {:?}", ai_output);

    info!("About to replace XML fragment content, this should trigger observer...");
    replace_xml_fragment_content(&doc, &fragment, &ai_output)?;
    info!(
        "XML fragment content replaced, transaction should have committed and triggered observer"
    );

    Ok((ai_output, doc))
}
