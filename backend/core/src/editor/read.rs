use std::sync::Arc;
use yrs::{Doc, GetString, Transact, XmlFragment};

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
    txn: &yrs::Transaction,
    output: &mut String,
    is_inline: bool,
) {
    match node {
        yrs::types::xml::XmlOut::Text(text_node) => {
            handle_text_node(text_node, txn, output);
        }
        yrs::types::xml::XmlOut::Element(element_node) => {
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
    txn: &yrs::Transaction,
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
    txn: &yrs::Transaction,
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
    txn: &yrs::Transaction,
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
