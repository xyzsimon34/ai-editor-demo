use anyhow::Result;
use std::sync::Arc;
use yrs::types::xml::XmlElementRef;
use yrs::{Doc, GetString, ReadTxn, Transact, XmlFragment, Xml};

// ============================================================================
// Constants: Element Type Definitions
// ============================================================================

/// 區塊級元素列表：這些元素在結束時需要添加換行符
const BLOCK_ELEMENTS: &[&str] = &[
    "paragraph",
    "heading",
    "code_block",
    "blockquote",
    "horizontal_rule",
];

/// 換行元素列表：這些元素本身代表換行
const BREAK_ELEMENTS: &[&str] = &["hard_break", "br"];

// ============================================================================
// Public API
// ============================================================================

/// 從 Yrs Doc 中提取純文字內容
///
/// 這個函數會遍歷文檔的 XML Fragment 結構，遞迴提取所有文字節點，
/// 並根據元素類型適當地添加換行符。
///
/// # Arguments
/// * `doc` - 共享的 Yrs Doc 實例，包含協作編輯的文檔內容
///
/// # Returns
/// 提取的純文字內容，已移除末尾多餘的換行符
///
/// # Example
/// ```rust
/// use std::sync::Arc;
/// use yrs::Doc;
/// use backend_core::editor::get_doc_content;
///
/// let doc = Arc::new(Doc::new());
/// let content = get_doc_content(&doc);
/// ```
pub fn get_doc_content(doc: &Arc<Doc>) -> String {
    let xml_fragment = doc.get_or_insert_xml_fragment("content");
    let txn = doc.transact();
    extract_text_from_fragment(&xml_fragment, &txn)
}

pub fn get_text_refs_in_paragraph(
    doc: &Arc<Doc>,
    paragraph_index: u32,
) -> Result<Vec<yrs::XmlTextRef>> {
    let xml_fragment = doc.get_or_insert_xml_fragment("content");
    let txn = doc.transact();

    let Some(child) = xml_fragment.get(&txn, paragraph_index) else {
        return Err(anyhow::anyhow!("No element at index {}", paragraph_index));
    };

    let yrs::types::xml::XmlOut::Element(para) = child else {
        return Err(anyhow::anyhow!(
            "Element at index {} is not a Element",
            paragraph_index
        ));
    };

    if para.tag().as_ref() != "paragraph" {
        return Err(anyhow::anyhow!(
            "Element at index {} is not a paragraph (tag: {})",
            paragraph_index,
            para.tag().as_ref()
        ));
    }

    let mut text_refs = Vec::new();
    collect_text_nodes_from_elem(&txn, &para, &mut text_refs);
    Ok(text_refs)
}

pub fn get_doc_xml_structure(doc: &Arc<Doc>) -> yrs::types::xml::XmlOut {
    let xml_fragment = doc.get_or_insert_xml_fragment("content");
    yrs::types::xml::XmlOut::Fragment(xml_fragment)
}

/// Debug 整個 doc 的結構
///
/// 返回一個可以用於 debug 打印的字符串表示
pub fn debug_doc_structure(doc: &Arc<Doc>) -> String {
    let xml_fragment = doc.get_or_insert_xml_fragment("content");
    let txn = doc.transact();
    let len = xml_fragment.len(&txn);

    let mut result = String::from("Doc structure:\n");
    result.push_str(&format!("Fragment length: {}\n", len));

    for i in 0..len {
        if let Some(child) = xml_fragment.get(&txn, i) {
            result.push_str(&format!("  [{}]: ", i));
            match &child {
                yrs::types::xml::XmlOut::Text(text_node) => {
                    let text = text_node.get_string(&txn);
                    result.push_str(&format!("Text({} chars): \"{}\"\n", text.len(), text));
                }
                yrs::types::xml::XmlOut::Element(elem) => {
                    let tag = elem.tag().as_ref();
                    let child_count = elem.len(&txn);
                    result.push_str(&format!("Element<{}> ({} children)\n", tag, child_count));

                    // 遞迴打印子節點
                    for j in 0..child_count {
                        if let Some(child) = elem.get(&txn, j) {
                            match &child {
                                yrs::types::xml::XmlOut::Text(text_node) => {
                                    let text = text_node.get_string(&txn);
                                    result.push_str(&format!(
                                        "    [{}]: Text({} chars): \"{}\"\n",
                                        j,
                                        text.len(),
                                        text
                                    ));
                                }
                                yrs::types::xml::XmlOut::Element(child_elem) => {
                                    let child_tag = child_elem.tag().as_ref();
                                    let grandchild_count = child_elem.len(&txn);
                                    result.push_str(&format!(
                                        "    [{}]: Element<{}> ({} children)\n",
                                        j, child_tag, grandchild_count
                                    ));
                                }
                                _ => {
                                    result.push_str(&format!("    [{}]: Other\n", j));
                                }
                            }
                        }
                    }
                }
                yrs::types::xml::XmlOut::Fragment(_) => {
                    result.push_str("Fragment\n");
                }
            }
        }
    }

    result
}

// ============================================================================
// Internal Implementation: Text Extraction
// ============================================================================

/// 從 XML Fragment 中提取所有文字內容
///
/// 遍歷 fragment 的所有子節點，遞迴提取文字，最後清理末尾多餘的換行符。
fn extract_text_from_fragment(
    fragment: &yrs::types::xml::XmlFragmentRef,
    txn: &yrs::Transaction,
) -> String {
    let mut content = String::new();
    let child_count = fragment.len(txn);

    // 遍歷所有子節點並提取文字
    for i in 0..child_count {
        if let Some(child) = fragment.get(txn, i) {
            extract_text_from_node(&child, txn, &mut content, false);
        }
    }

    // 移除末尾多餘的換行符，保持輸出整潔
    content.trim_end_matches('\n').to_string()
}

/// 從單個 XML 節點遞迴提取文字內容
///
/// 根據節點類型（Text、Element、Fragment）採用不同的處理策略：
/// - Text: 直接提取文字
/// - Element: 遞迴處理子節點，並根據元素類型添加換行
/// - Fragment: 遞迴處理嵌套的 fragment
///
/// # Arguments
/// * `node` - 要處理的 XML 節點
/// * `txn` - 只讀事務
/// * `output` - 輸出緩衝區，累積提取的文字
/// * `is_inline` - 標記當前是否在 inline 上下文中（用於控制換行行為）
fn extract_text_from_node(
    node: &yrs::types::xml::XmlOut,
    txn: &impl yrs::ReadTxn,
    output: &mut String,
    is_inline: bool,
) {
    match node {
        yrs::types::xml::XmlOut::Text(text_node) => {
            handle_text_node(text_node, txn, output);
        }
        yrs::types::xml::XmlOut::Element(element_node) => {
            // Check if this is an AI suggestion element before processing
            // Skip AI suggestion elements and all their content
            let tag_name = element_node.tag().as_ref();
            if tag_name == "span" {
                let attrs = element_node.attributes(txn);
                let mut is_ai_suggestion = false;
                for (key, value) in attrs {
                    if key == "data-type" && value.to_string(txn) == "ai-suggestion" {
                        is_ai_suggestion = true;
                        break;
                    }
                }
                if is_ai_suggestion {
                    // Skip this element and all its content
                    return;
                }
            }
            handle_element_node(element_node, txn, output, is_inline);
        }
        yrs::types::xml::XmlOut::Fragment(fragment_node) => {
            handle_fragment_node(fragment_node, txn, output, is_inline);
        }
    }
}

// ============================================================================
// Node Type Handlers
// ============================================================================

/// 處理文字節點：直接提取文字內容
fn handle_text_node(
    text_node: &yrs::types::xml::XmlTextRef,
    txn: &impl yrs::ReadTxn,
    output: &mut String,
) {
    let text = text_node.get_string(txn);
    if !text.is_empty() {
        output.push_str(&text);
    }
}

/// 處理元素節點：遞迴處理子節點，並根據元素類型添加換行
fn handle_element_node(
    element_node: &yrs::types::xml::XmlElementRef,
    txn: &impl yrs::ReadTxn,
    output: &mut String,
    is_inline: bool,
) {
    let tag_name = element_node.tag().as_ref();
    let child_count = element_node.len(txn);

    // 判斷元素類型
    let is_block_element = is_block_level_element(tag_name);
    let is_break_element = is_break_element(tag_name);

    // 遞迴處理所有子節點
    // 如果當前是區塊級元素，子節點會被標記為 inline（避免重複換行）
    for i in 0..child_count {
        if let Some(child) = element_node.get(txn, i) {
            extract_text_from_node(&child, txn, output, !is_block_element);
        }
    }

    // 根據元素類型添加換行符
    if is_break_element {
        // 換行元素：直接添加換行
        output.push('\n');
    } else if is_block_element && !is_inline {
        // 區塊級元素：在結束時添加換行（但不在嵌套的 inline 上下文中）
        output.push('\n');
    }
}

/// 處理 Fragment 節點：遞迴處理嵌套的 fragment
fn handle_fragment_node(
    fragment_node: &yrs::types::xml::XmlFragmentRef,
    txn: &impl yrs::ReadTxn,
    output: &mut String,
    is_inline: bool,
) {
    let child_count = fragment_node.len(txn);
    for i in 0..child_count {
        if let Some(child) = fragment_node.get(txn, i) {
            extract_text_from_node(&child, txn, output, is_inline);
        }
    }
}

// ============================================================================
// Element Type Helpers
// ============================================================================

/// 判斷是否為區塊級元素
///
/// 區塊級元素（如 paragraph、heading）在結束時需要添加換行符。
fn is_block_level_element(tag_name: &str) -> bool {
    BLOCK_ELEMENTS.contains(&tag_name)
}

/// 判斷是否為換行元素
///
/// 換行元素（如 hard_break、br）本身代表換行，需要直接添加換行符。
fn is_break_element(tag_name: &str) -> bool {
    BREAK_ELEMENTS.contains(&tag_name)
}

// ============================================================================
// Text Node Collection Helpers
// ============================================================================

/// Helper: Recursively find all XmlTextRef nodes in an element
/// Uses ReadTxn trait so it works with both Transaction and TransactionMut
pub fn collect_text_nodes_from_elem(
    txn: &impl yrs::ReadTxn,
    elem: &yrs::XmlElementRef,
    collector: &mut Vec<yrs::XmlTextRef>,
) {
    use yrs::types::xml::XmlOut;

    let len = elem.len(txn);
    for i in 0..len {
        if let Some(child) = elem.get(txn, i) {
            match child {
                XmlOut::Element(child_elem) => {
                    collect_text_nodes_from_elem(txn, &child_elem, collector);
                }
                XmlOut::Text(text_ref) => {
                    collector.push(text_ref);
                }
                _ => {}
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use yrs::XmlTextPrelim;

    #[test]
    fn test_get_doc_content_empty() {
        let doc = Arc::new(Doc::new());
        let text = get_doc_content(&doc);
        assert_eq!(text, "");
    }

    #[test]
    fn test_get_doc_content_with_text() {
        let doc = Arc::new(Doc::new());
        let fragment = doc.get_or_insert_xml_fragment("content");

        // 創建可寫事務並插入文字
        {
            let mut txn = doc.transact_mut();
            fragment.insert(&mut txn, 0, XmlTextPrelim::new("hello, world!"));
        } // 事務在這裡結束

        // 使用只讀事務讀取內容
        let text = get_doc_content(&doc);
        assert_eq!(text, "hello, world!");
    }

    #[test]
    fn test_extract_text_from_fragment_empty() {
        let doc = Doc::new();
        let xml_fragment = doc.get_or_insert_xml_fragment("content");
        let txn = doc.transact();
        let text = extract_text_from_fragment(&xml_fragment, &txn);
        assert_eq!(text, "");
    }

    #[test]
    fn test_extract_text_from_fragment_with_text() {
        let doc = Doc::new();
        let fragment = doc.get_or_insert_xml_fragment("content");

        // 插入文字內容
        {
            let mut txn = doc.transact_mut();
            fragment.insert(&mut txn, 0, XmlTextPrelim::new("hello, world!"));
        }

        // 提取文字
        let txn = doc.transact();
        let text = extract_text_from_fragment(&fragment, &txn);
        assert_eq!(text, "hello, world!");
    }

    #[test]
    fn test_is_block_level_element() {
        assert!(is_block_level_element("paragraph"));
        assert!(is_block_level_element("heading"));
        assert!(is_block_level_element("code_block"));
        assert!(!is_block_level_element("span"));
        assert!(!is_block_level_element("strong"));
    }

    #[test]
    fn test_is_break_element() {
        assert!(is_break_element("hard_break"));
        assert!(is_break_element("br"));
        assert!(!is_break_element("paragraph"));
    }
}
