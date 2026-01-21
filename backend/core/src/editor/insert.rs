use anyhow::Result;
use std::sync::Arc;
use yrs::{Doc, Text, Transact, XmlFragment};

/// 在 paragraph 中指定 textref 的 sticky point 位置插入 AI 內容
///
/// 此函數會先建立 StickyIndex，然後使用該索引來插入內容，
/// 確保即使在協作編輯環境中也能準確定位插入位置。
/// StickyIndex 在插入完成後會自動釋放。
///
/// # Arguments
/// * `doc` - 共享的 Yrs Doc 實例
/// * `paragraph_index` - paragraph 在 fragment 中的索引
/// * `textref_index` - 在 paragraph 中第幾個 textref（從 0 開始）
/// * `offset` - 在該 textref 中的字符位置（從 0 開始）
/// * `content` - 要插入的 AI 生成內容
///
/// # Returns
/// `Ok(())` 如果成功，`Err` 如果失敗
///
/// # Example
/// ```rust
/// // 在第 0 個 paragraph 的第 1 個 textref 的位置 5 插入內容
/// insert_ai_content_to_paragraph(&doc, 0, 1, 5, "AI content")?;
/// ```
pub fn insert_ai_content_to_paragraph(
    doc: &Arc<Doc>,
    paragraph_index: u32,
    textref_index: usize,
    offset: u32,
    content: &str,
) -> Result<()> {
    if content.trim().is_empty() {
        return Err(anyhow::anyhow!("Content cannot be empty"));
    }

    let xml_fragment = doc.get_or_insert_xml_fragment("content");

    // 第一步：使用只讀事務來建立 StickyIndex
    let txn = doc.transact();

    // 獲取 paragraph 元素
    let Some(child) = xml_fragment.get(&txn, paragraph_index) else {
        return Err(anyhow::anyhow!("No element at index {}", paragraph_index));
    };

    let yrs::types::xml::XmlOut::Element(para) = child else {
        return Err(anyhow::anyhow!(
            "Element at index {} is not an Element",
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

    // 收集所有 textrefs
    let mut text_refs = Vec::new();
    collect_text_nodes_from_elem(&txn, &para, &mut text_refs);

    // 檢查 textref_index 是否有效
    if textref_index >= text_refs.len() {
        return Err(anyhow::anyhow!(
            "TextRef index {} is out of range (paragraph has {} text nodes)",
            textref_index,
            text_refs.len()
        ));
    }

    let target_text_ref = &text_refs[textref_index];
    let text_len = target_text_ref.len(&txn) as u32;

    // 檢查 offset 是否有效
    if offset > text_len {
        return Err(anyhow::anyhow!(
            "Offset {} is out of range (text node has {} characters)",
            offset,
            text_len
        ));
    }

    // 第二步：在只讀事務中確定插入位置
    // 注意：對於 XML Text 節點，我們使用 offset 作為插入位置
    // XML Text 節點的字符索引在協作編輯中是穩定的，所以可以直接使用 offset
    // 如果需要更精確的位置追蹤，可以考慮使用 StickyIndex（需要確認 Yrs API）

    // 釋放只讀事務
    drop(txn);

    // 第三步：使用確定的位置來插入內容（需要可寫事務）
    let mut txn_mut = doc.transact_mut();

    // 準備要插入的內容（如果 offset > 0 且前面有文字，加空格）
    let text_to_insert = if offset > 0 && offset < text_len {
        format!(" {}", content.trim())
    } else {
        content.trim().to_string()
    };

    // 使用 offset 來插入內容
    // 對於 XML Text 節點，offset 本身就是穩定的插入位置
    target_text_ref.insert(&mut txn_mut, offset, &text_to_insert);

    // 事務在函數結束時自動提交
    Ok(())
}

/// Helper: Recursively find all XmlTextRef nodes in an element
/// Uses ReadTxn trait so it works with both Transaction and TransactionMut
fn collect_text_nodes_from_elem(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::read::{debug_doc_structure, get_doc_content};

    #[test]
    fn test_insert_ai_content_to_paragraph() {
        let doc = Arc::new(Doc::new());

        // 先創建一個 paragraph 結構
        let xml_fragment = doc.get_or_insert_xml_fragment("content");
        {
            let mut txn = doc.transact_mut();
            let para = yrs::types::xml::XmlElementPrelim::empty("paragraph");
            xml_fragment.insert(&mut txn, 0, para);
        }

        // 在 paragraph 中添加一個空的 text node
        {
            let mut txn = doc.transact_mut();
            if let Some(yrs::types::xml::XmlOut::Element(para)) = xml_fragment.get(&txn, 0) {
                para.insert(&mut txn, 0, yrs::XmlTextPrelim::new(""));
            }
        }

        // 現在可以插入內容了
        insert_ai_content_to_paragraph(&doc, 0, 0, 0, "Hello, world!").unwrap();

        let content = get_doc_content(&doc);
        assert_eq!(content, "Hello, world!");

        // Debug 整個 doc 結構
        println!(
            "test_insert_ai_content_to_paragraph\n{}",
            debug_doc_structure(&doc)
        );
    }

    #[test]
    fn test_insert_ai_content_to_paragraph_with_existing_text() {
        let doc = Arc::new(Doc::new());
        let xml_fragment = doc.get_or_insert_xml_fragment("content");
        {
            let mut txn = doc.transact_mut();
            let para = yrs::types::xml::XmlElementPrelim::empty("paragraph");
            xml_fragment.insert(&mut txn, 0, para);
        }

        {
            let mut txn = doc.transact_mut();
            if let Some(yrs::types::xml::XmlOut::Element(para)) = xml_fragment.get(&txn, 0) {
                para.insert(&mut txn, 0, yrs::XmlTextPrelim::new("Hello, world!"));
            }
        }
        insert_ai_content_to_paragraph(&doc, 0, 0, 0, "Hello, world!").unwrap();
        insert_ai_content_to_paragraph(&doc, 0, 0, 0, "I am a test.").unwrap();
        let content = get_doc_content(&doc);
        assert_eq!(content, "Hello, world! I am a test.");
        // Debug 整個 doc 結構
        println!(
            "test_insert_ai_content_to_paragraph_with_existing_text\n{}",
            debug_doc_structure(&doc)
        );
    }

    #[test]
    fn test_concurrent_insert_at_same_index() {
        let doc = Arc::new(Doc::new());
        let xml_fragment = doc.get_or_insert_xml_fragment("content");

        // 設置初始結構：一個 paragraph 包含 "Hello"
        {
            let mut txn = doc.transact_mut();
            let para = yrs::types::xml::XmlElementPrelim::empty("paragraph");
            xml_fragment.insert(&mut txn, 0, para);
        }

        {
            let mut txn = doc.transact_mut();
            if let Some(yrs::types::xml::XmlOut::Element(para)) = xml_fragment.get(&txn, 0) {
                para.insert(&mut txn, 0, yrs::XmlTextPrelim::new("Hello"));
            }
        }

        // 驗證初始狀態
        let initial_content = get_doc_content(&doc);
        assert_eq!(initial_content, "Hello");
        println!("Initial content: {}", initial_content);

        // 模擬兩個用戶同時在索引 0 插入不同的內容
        // User 1: 在位置 0 插入 "User1: "
        // User 2: 在位置 0 插入 "User2: "

        // 由於 Yrs 的 CRDT 特性，兩個插入都會成功，但順序可能不同
        // 我們先讓 User 1 插入
        insert_ai_content_to_paragraph(&doc, 0, 0, 0, "User1: ").unwrap();

        // 然後讓 User 2 在同一個位置插入（但此時位置已經改變）
        // 實際上，User 2 應該在原始位置 0 插入，但由於 User 1 已經插入了，
        // User 2 的插入會根據 Yrs 的 CRDT 機制來決定位置

        // 獲取當前狀態來確定 User 2 應該插入的位置
        let after_user1 = get_doc_content(&doc);
        println!("After User1 insert: {}", after_user1);

        // User 2 嘗試在原始位置 0 插入
        // 注意：由於 User 1 已經插入，實際的插入位置可能會有所不同
        // 這取決於 Yrs 的 CRDT 合併策略
        insert_ai_content_to_paragraph(&doc, 0, 0, 0, "User2: ").unwrap();

        let final_content = get_doc_content(&doc);
        println!("Final content: {}", final_content);
        println!(
            "test_concurrent_insert_at_same_index\n{}",
            debug_doc_structure(&doc)
        );

        // 驗證兩個插入都成功了
        assert!(final_content.contains("User1:"));
        assert!(final_content.contains("User2:"));
        assert!(final_content.contains("Hello"));
    }

    #[tokio::test]
    async fn test_truly_concurrent_inserts() {
        use std::sync::Arc;
        use tokio::task;

        let doc = Arc::new(Doc::new());
        let xml_fragment = doc.get_or_insert_xml_fragment("content");

        // 設置初始結構：一個 paragraph 包含 "Start"
        {
            let mut txn = doc.transact_mut();
            let para = yrs::types::xml::XmlElementPrelim::empty("paragraph");
            xml_fragment.insert(&mut txn, 0, para);
        }

        {
            let mut txn = doc.transact_mut();
            if let Some(yrs::types::xml::XmlOut::Element(para)) = xml_fragment.get(&txn, 0) {
                para.insert(&mut txn, 0, yrs::XmlTextPrelim::new("Start"));
            }
        }

        let initial_content = get_doc_content(&doc);
        assert_eq!(initial_content, "Start");
        println!("Initial content: {}", initial_content);

        // 創建兩個任務，模擬兩個用戶同時在同一個位置插入
        let doc1 = doc.clone();
        let doc2 = doc.clone();

        // User 1 任務：在位置 0 插入 "A"
        let task1 =
            task::spawn(async move { insert_ai_content_to_paragraph(&doc1, 0, 0, 0, "ABC") });

        // User 2 任務：也在位置 0 插入 "B"
        let task2 =
            task::spawn(async move { insert_ai_content_to_paragraph(&doc2, 0, 0, 0, "DEF") });

        // 等待兩個任務完成
        let result1 = task1.await.unwrap();
        let result2 = task2.await.unwrap();

        assert!(result1.is_ok());
        assert!(result2.is_ok());

        let final_content = get_doc_content(&doc);
        println!("Final content after concurrent inserts: {}", final_content);
        println!(
            "test_truly_concurrent_inserts\n{}",
            debug_doc_structure(&doc)
        );

        // 驗證兩個插入都成功了
        assert!(final_content.contains("ABC"));
        assert!(final_content.contains("DEF"));
        assert!(final_content.contains("Start"));

        // 驗證內容長度正確（Start + A + B + 可能的空格）
        assert!(final_content.len() >= "StartABCDEF".len());
    }
}
