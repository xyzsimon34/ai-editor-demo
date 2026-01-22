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

fn replace_xml_fragment_content(
    doc: &Doc,
    fragment: &XmlFragmentRef,
    new_xml: &str,
    run_id: &str,
) -> Result<()> {
    let mut txn = doc.transact_mut();

    // Get the first element from fragment
    if let Some(first_child) = fragment.get(&txn, 0) {
        if let yrs::types::xml::XmlOut::Element(elem) = first_child {
            // Clear existing content of the first element
            let len = elem.len(&txn);
            if len > 0 {
                elem.remove_range(&mut txn, 0, len);
            }

            // Parse and insert new XML as text nodes
            let parsed = parse_xml_string(new_xml, run_id)?;
            for prelim in &parsed {
                match prelim {
                    XmlPrelim::Text(text) => {
                        let len = elem.len(&txn);
                        elem.insert(&mut txn, len, yrs::XmlTextPrelim::new(text));
                    }
                    XmlPrelim::Element { .. } => {
                        // Skip elements, only insert text
                    }
                }
            }
        }
    }

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

fn parse_xml_string(xml: &str, run_id: &str) -> Result<Vec<XmlPrelim>> {
    let mut result = Vec::new();
    let mut chars = xml.chars().peekable();
    let mut current_text = String::new();

    while chars.peek().is_some() {
        if *chars.peek().unwrap() == '<' {
            // If we have accumulated text, save it
            if !current_text.is_empty() {
                result.push(XmlPrelim::Text(current_text.clone()));
                current_text.clear();
            }

            // Parse the entire tag as text (including opening tag, content, and closing tag)
            let tag_text = parse_tag_as_text(&mut chars, run_id);
            if !tag_text.is_empty() {
                result.push(XmlPrelim::Text(tag_text));
            }
        } else {
            current_text.push(chars.next().unwrap());
        }
    }

    // Add any remaining text
    if !current_text.is_empty() {
        result.push(XmlPrelim::Text(current_text));
    }

    Ok(result)
}

fn parse_tag_as_text(chars: &mut std::iter::Peekable<std::str::Chars>, run_id: &str) -> String {
    let mut text = String::new();
    let mut original_tag = String::new();

    // Parse opening tag: <tag...>
    if chars.peek() == Some(&'<') {
        original_tag.push(chars.next().unwrap()); // '<'

        // Read until '>'
        while let Some(&ch) = chars.peek() {
            original_tag.push(chars.next().unwrap());
            if ch == '>' {
                break;
            }
        }

        // Check if it's a self-closing tag
        if original_tag.ends_with("/>") {
            // Check if it's a <del> tag and convert it
            if original_tag.starts_with("<del") {
                return format!(
                    "<aisuggestion tool=\"linter\" status=\"pending\" runid=\"{}\" operation=\"delete\"/>",
                    run_id
                );
            }
            // Check if it's a <ins> tag and convert it
            if original_tag.starts_with("<ins") {
                return format!(
                    "<aisuggestion tool=\"linter\" status=\"pending\" runid=\"{}\" operation=\"insert\"/>",
                    run_id
                );
            }
            return original_tag;
        }

        // Check if it's a <del> or <ins> tag
        let is_del_tag = original_tag.starts_with("<del") && !original_tag.starts_with("<del/");
        let is_ins_tag = original_tag.starts_with("<ins") && !original_tag.starts_with("<ins/");

        if is_del_tag {
            // Start building the <aisuggestion> tag with delete operation
            text.push_str(&format!(
                "<aisuggestion tool=\"linter\" status=\"pending\" runid=\"{}\" operation=\"delete\">",
                run_id
            ));
        } else if is_ins_tag {
            // Start building the <aisuggestion> tag with insert operation
            text.push_str(&format!(
                "<aisuggestion tool=\"linter\" status=\"pending\" runid=\"{}\" operation=\"insert\">",
                run_id
            ));
        } else {
            text.push_str(&original_tag);
        }

        // Parse content and closing tag
        let mut depth = 1;
        while depth > 0 && chars.peek().is_some() {
            if chars.peek() == Some(&'<') {
                // Check if it's a closing tag
                let peeked: Vec<_> = chars.clone().take(2).collect();
                if peeked.len() == 2 && peeked[1] == '/' {
                    // Closing tag: </tag>
                    let mut closing_tag = String::new();
                    closing_tag.push(chars.next().unwrap()); // '<'
                    closing_tag.push(chars.next().unwrap()); // '/'

                    // Read until '>'
                    while let Some(&ch) = chars.peek() {
                        closing_tag.push(chars.next().unwrap());
                        if ch == '>' {
                            depth -= 1;
                            break;
                        }
                    }

                    // If it was a <del> or <ins> tag, close with </aisuggestion>
                    if is_del_tag || is_ins_tag {
                        text.push_str("</aisuggestion>");
                    } else {
                        text.push_str(&closing_tag);
                    }
                } else {
                    // Nested opening tag
                    text.push(chars.next().unwrap()); // '<'
                    while let Some(&ch) = chars.peek() {
                        text.push(chars.next().unwrap());
                        if ch == '>' {
                            depth += 1;
                            break;
                        }
                    }
                }
            } else {
                // Regular content
                text.push(chars.next().unwrap());
            }
        }
    }

    text
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
            // Preserve text nodes, including whitespace-only ones between elements
            children.push(XmlPrelim::Text(text));
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
    info!("Original XML: {:?}", original_xml);
    let original_xml = original_xml
        .replace("<paragraph>", "")
        .replace("</paragraph>", "");
    // Generate a unique run ID for this AI generation
    let run_id = format!(
        "linter-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );

    let client = reqwest::Client::new();

    let system_content = r#"You are the "Linguistic Diff Engine." Your task is to correct grammar and spelling errors using <del> and <ins> tags to show exactly what changed.

        ### Operational Rules:
        1. **Markup Logic**: 
            - Use `<del>wrong</del>` for removed or incorrect text.
            - Use `<ins>correct</ins>` for added or corrected text.
            - Keep all original, unchanged text as plain text.
        2. **Linguistic Scope**: Fix grammar, spelling, punctuation, and capitalization.
        3. **Output Constraint**: 
            - Return ONLY the processed text string containing the tags.
            - Do NOT wrap the result in `<aisuggestion>`, Markdown code blocks, or any other tags.
            - Do NOT provide explanations or comments.

        ### Example Input:
        "hello woold"

        ### Example Output:
        Hello, <del>woold</del><ins>world</ins>"#;
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

    // Replace content with AI output
    info!("Linter response: {:?}", ai_output);

    info!("About to replace XML fragment content, this should trigger observer...");
    replace_xml_fragment_content(&doc, &fragment, &ai_output, &run_id)?;
    info!(
        "XML fragment content replaced, transaction should have committed and triggered observer"
    );

    Ok((ai_output, doc))
}
