use anyhow::{Context, Result};
use serde_json::json;
use std::sync::Arc;
use tracing::info;
use yrs::types::xml::{XmlElementRef, XmlFragmentRef};
use yrs::{Doc, GetString, Transact, Xml, XmlFragment};

use crate::llm::llm_model;

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
    info!("Parsed XML: {:?}", result);
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

                // Convert attributes to child elements
                for (key, value) in attrs {
                    let attr_elem_prelim = yrs::types::xml::XmlElementPrelim::empty(key.as_str());
                    let attr_elem = elem.insert(txn, elem.len(txn), attr_elem_prelim);
                    attr_elem.insert(txn, 0, yrs::XmlTextPrelim::new(value));
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

            // Convert attributes to child elements
            for (key, value) in attrs {
                let attr_elem_prelim = yrs::types::xml::XmlElementPrelim::empty(key.as_str());
                let attr_elem = child_elem.insert(txn, child_elem.len(txn), attr_elem_prelim);
                attr_elem.insert(txn, 0, yrs::XmlTextPrelim::new(value));
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
    let client = reqwest::Client::new();

    let system_content = r#"You are the "Schema Sentry," a specialized linguistic linter for Yjs structures.

Your sole purpose is to:
1. Identify grammatical errors, spelling mistakes, and areas for clarity improvement.
2. Mark changes using specific tags:
   - For text that should be REMOVED, wrap it in <del>original text</del>.
   - For text that should be ADDED, wrap it in <ins>new text</ins>.
3. To replace a word/phrase, use the sequence: <del>old</del><ins>new</ins>.
4. Keep all other original text and XML tag structures untouched.
5. Strict Constraints: 
   - Do NOT provide any explanations, comments, or markdown code blocks.
   - Output ONLY the complete, annotated XML string.
   - Ensure the content inside <del> matches the original text exactly.
6. If no errors are found, return the original XML string exactly as it is."#;

    let request_payload = json!({
        "model": llm_model::GPT4_O_MINI,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_string() {
        let result = parse_xml_string("").unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parse_whitespace_only() {
        let result = parse_xml_string("   \n\t  ").unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parse_pure_text() {
        let result = parse_xml_string("Hello world").unwrap();
        assert_eq!(result.len(), 1);
        match &result[0] {
            XmlPrelim::Text(text) => assert_eq!(text, "Hello world"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn test_parse_text_with_whitespace() {
        let result = parse_xml_string("  Hello world  ").unwrap();
        assert_eq!(result.len(), 1);
        match &result[0] {
            XmlPrelim::Text(text) => assert_eq!(text.trim(), "Hello world"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn test_parse_simple_element() {
        let result = parse_xml_string("<div></div>").unwrap();
        assert_eq!(result.len(), 1);
        match &result[0] {
            XmlPrelim::Element {
                tag,
                attrs,
                children,
            } => {
                assert_eq!(tag, "div");
                assert_eq!(attrs.len(), 0);
                assert_eq!(children.len(), 0);
            }
            _ => panic!("Expected Element variant"),
        }
    }

    #[test]
    fn test_parse_element_with_whitespace() {
        let result = parse_xml_string("  <div>  </div>  ").unwrap();
        assert_eq!(result.len(), 1);
        match &result[0] {
            XmlPrelim::Element { tag, .. } => assert_eq!(tag, "div"),
            _ => panic!("Expected Element variant"),
        }
    }

    #[test]
    fn test_parse_self_closing_tag() {
        let result = parse_xml_string("<br/>").unwrap();
        assert_eq!(result.len(), 1);
        match &result[0] {
            XmlPrelim::Element {
                tag,
                attrs,
                children,
            } => {
                assert_eq!(tag, "br");
                assert_eq!(attrs.len(), 0);
                assert_eq!(children.len(), 0);
            }
            _ => panic!("Expected Element variant"),
        }
    }

    #[test]
    fn test_parse_self_closing_tag_with_whitespace() {
        let result = parse_xml_string("<br />").unwrap();
        assert_eq!(result.len(), 1);
        match &result[0] {
            XmlPrelim::Element { tag, .. } => assert_eq!(tag, "br"),
            _ => panic!("Expected Element variant"),
        }
    }

    #[test]
    fn test_parse_element_with_single_attribute() {
        let result = parse_xml_string(r#"<div id="test"></div>"#).unwrap();
        assert_eq!(result.len(), 1);
        match &result[0] {
            XmlPrelim::Element { tag, attrs, .. } => {
                assert_eq!(tag, "div");
                assert_eq!(attrs.len(), 1);
                assert_eq!(attrs[0], ("id".to_string(), "test".to_string()));
            }
            _ => panic!("Expected Element variant"),
        }
    }

    #[test]
    fn test_parse_element_with_multiple_attributes() {
        let result = parse_xml_string(r#"<div id="test" class="container"></div>"#).unwrap();
        assert_eq!(result.len(), 1);
        match &result[0] {
            XmlPrelim::Element { tag, attrs, .. } => {
                assert_eq!(tag, "div");
                assert_eq!(attrs.len(), 2);
                assert_eq!(attrs[0], ("id".to_string(), "test".to_string()));
                assert_eq!(attrs[1], ("class".to_string(), "container".to_string()));
            }
            _ => panic!("Expected Element variant"),
        }
    }

    #[test]
    fn test_parse_element_with_text_child() {
        let result = parse_xml_string("<div>Hello</div>").unwrap();
        assert_eq!(result.len(), 1);
        match &result[0] {
            XmlPrelim::Element { tag, children, .. } => {
                assert_eq!(tag, "div");
                assert_eq!(children.len(), 1);
                match &children[0] {
                    XmlPrelim::Text(text) => assert_eq!(text, "Hello"),
                    _ => panic!("Expected Text child"),
                }
            }
            _ => panic!("Expected Element variant"),
        }
    }

    #[test]
    fn test_parse_element_with_text_and_whitespace() {
        let result = parse_xml_string("<div>  Hello  </div>").unwrap();
        assert_eq!(result.len(), 1);
        match &result[0] {
            XmlPrelim::Element { children, .. } => {
                assert_eq!(children.len(), 1);
                match &children[0] {
                    XmlPrelim::Text(text) => assert_eq!(text.trim(), "Hello"),
                    _ => panic!("Expected Text child"),
                }
            }
            _ => panic!("Expected Element variant"),
        }
    }

    #[test]
    fn test_parse_nested_elements() {
        let result = parse_xml_string("<div><span>Hello</span></div>").unwrap();
        assert_eq!(result.len(), 1);
        match &result[0] {
            XmlPrelim::Element { tag, children, .. } => {
                assert_eq!(tag, "div");
                assert_eq!(children.len(), 1);
                match &children[0] {
                    XmlPrelim::Element { tag, children, .. } => {
                        assert_eq!(tag, "span");
                        assert_eq!(children.len(), 1);
                        match &children[0] {
                            XmlPrelim::Text(text) => assert_eq!(text, "Hello"),
                            _ => panic!("Expected Text in nested element"),
                        }
                    }
                    _ => panic!("Expected nested Element"),
                }
            }
            _ => panic!("Expected Element variant"),
        }
    }

    #[test]
    fn test_parse_multiple_root_elements() {
        let result = parse_xml_string("<div></div><span></span>").unwrap();
        assert_eq!(result.len(), 2);
        match &result[0] {
            XmlPrelim::Element { tag, .. } => assert_eq!(tag, "div"),
            _ => panic!("Expected Element"),
        }
        match &result[1] {
            XmlPrelim::Element { tag, .. } => assert_eq!(tag, "span"),
            _ => panic!("Expected Element"),
        }
    }

    #[test]
    fn test_parse_mixed_content_text_and_elements() {
        let result = parse_xml_string("Before<div></div>After").unwrap();
        assert_eq!(result.len(), 3);
        match &result[0] {
            XmlPrelim::Text(text) => assert_eq!(text, "Before"),
            _ => panic!("Expected Text"),
        }
        match &result[1] {
            XmlPrelim::Element { tag, .. } => assert_eq!(tag, "div"),
            _ => panic!("Expected Element"),
        }
        match &result[2] {
            XmlPrelim::Text(text) => assert_eq!(text, "After"),
            _ => panic!("Expected Text"),
        }
    }

    #[test]
    fn test_parse_element_with_mixed_children() {
        let result = parse_xml_string("<div>Text1<span></span>Text2</div>").unwrap();
        assert_eq!(result.len(), 1);
        match &result[0] {
            XmlPrelim::Element { children, .. } => {
                assert_eq!(children.len(), 3);
                match &children[0] {
                    XmlPrelim::Text(text) => assert_eq!(text, "Text1"),
                    _ => panic!("Expected Text"),
                }
                match &children[1] {
                    XmlPrelim::Element { tag, .. } => assert_eq!(tag, "span"),
                    _ => panic!("Expected Element"),
                }
                match &children[2] {
                    XmlPrelim::Text(text) => assert_eq!(text, "Text2"),
                    _ => panic!("Expected Text"),
                }
            }
            _ => panic!("Expected Element"),
        }
    }

    #[test]
    fn test_parse_tag_with_hyphen() {
        let result = parse_xml_string("<my-tag></my-tag>").unwrap();
        assert_eq!(result.len(), 1);
        match &result[0] {
            XmlPrelim::Element { tag, .. } => assert_eq!(tag, "my-tag"),
            _ => panic!("Expected Element"),
        }
    }

    #[test]
    fn test_parse_tag_with_underscore() {
        let result = parse_xml_string("<my_tag></my_tag>").unwrap();
        assert_eq!(result.len(), 1);
        match &result[0] {
            XmlPrelim::Element { tag, .. } => assert_eq!(tag, "my_tag"),
            _ => panic!("Expected Element"),
        }
    }

    #[test]
    fn test_parse_attribute_with_special_chars() {
        let result = parse_xml_string(r#"<div data-value="test-value"></div>"#).unwrap();
        assert_eq!(result.len(), 1);
        match &result[0] {
            XmlPrelim::Element { attrs, .. } => {
                assert_eq!(attrs[0].1, "test-value");
            }
            _ => panic!("Expected Element"),
        }
    }

    #[test]
    fn test_parse_empty_text_nodes_ignored() {
        let result = parse_xml_string("<div>   </div>").unwrap();
        assert_eq!(result.len(), 1);
        match &result[0] {
            XmlPrelim::Element { children, .. } => {
                assert_eq!(children.len(), 0);
            }
            _ => panic!("Expected Element"),
        }
    }

    #[test]
    fn test_parse_complex_nested_structure() {
        let result =
            parse_xml_string(r#"<div id="outer"><span class="inner">Text</span><p>Para</p></div>"#)
                .unwrap();
        assert_eq!(result.len(), 1);
        match &result[0] {
            XmlPrelim::Element {
                tag,
                attrs,
                children,
            } => {
                assert_eq!(tag, "div");
                assert_eq!(attrs.len(), 1);
                assert_eq!(attrs[0].0, "id");
                assert_eq!(attrs[0].1, "outer");
                assert_eq!(children.len(), 2);
            }
            _ => panic!("Expected Element"),
        }
    }

    #[test]
    fn test_parse_malformed_missing_closing_tag() {
        let result = parse_xml_string("<div>");
        assert!(result.is_err() || result.unwrap().is_empty());
    }

    #[test]
    fn test_parse_malformed_mismatched_tags() {
        let result = parse_xml_string("<div></span>");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_malformed_unclosed_attribute() {
        let result = parse_xml_string(r#"<div id="test></div>"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_element_with_attributes() {
        let result = parse_xml_string(
            r#"<paragraph id="p1" class="content">
Yesterday I
<del>go</del>
<ins>went</ins>
to the store and I
<del>buying</del>
<ins>bought</ins>
</paragraph>"#,
        )
        .unwrap();
        assert_eq!(result.len(), 1);
        match &result[0] {
            XmlPrelim::Element {
                tag,
                attrs,
                children,
            } => {
                assert_eq!(tag, "paragraph");
                assert_eq!(attrs.len(), 2);
                assert_eq!(attrs[0], ("id".to_string(), "p1".to_string()));
                assert_eq!(attrs[1], ("class".to_string(), "content".to_string()));
                // Verify children: Text, del, ins, Text, del, ins
                assert_eq!(children.len(), 6);
                match &children[0] {
                    XmlPrelim::Text(text) => assert!(text.contains("Yesterday I")),
                    _ => panic!("Expected Text at children[0]"),
                }
                match &children[1] {
                    XmlPrelim::Element { tag, .. } => assert_eq!(tag, "del"),
                    _ => panic!("Expected del Element"),
                }
                match &children[2] {
                    XmlPrelim::Element { tag, .. } => assert_eq!(tag, "ins"),
                    _ => panic!("Expected ins Element"),
                }
                match &children[3] {
                    XmlPrelim::Text(text) => assert!(text.contains("to the store")),
                    _ => panic!("Expected Text at children[3]"),
                }
                match &children[4] {
                    XmlPrelim::Element { tag, .. } => assert_eq!(tag, "del"),
                    _ => panic!("Expected del Element"),
                }
                match &children[5] {
                    XmlPrelim::Element { tag, .. } => assert_eq!(tag, "ins"),
                    _ => panic!("Expected ins Element"),
                }
            }
            _ => panic!("Expected Element"),
        }
    }
}
