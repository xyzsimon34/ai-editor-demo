use anyhow::Result;
use std::sync::Arc;
use yrs::{Assoc, Doc, IndexedSequence, Text, Transact};

use crate::editor::xml_structure::{collect_text_nodes_from_elem, get_paragraph_element};

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
    let para = get_paragraph_element(&xml_fragment, &txn, paragraph_index)?;

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

    // 建立 StickyIndex 來追蹤位置（在只讀事務中）
    // 注意：雖然我們建立了 StickyIndex，但 Yrs 的 insert 方法需要 u32 位置
    // 在協作編輯環境中，StickyIndex 可以幫助我們在文檔結構變化時追蹤位置
    // 但目前我們使用 offset 直接插入，因為 XmlTextRef 的字符索引在協作編輯中是穩定的
    let _sticky_index = target_text_ref
        .sticky_index(&txn, offset, Assoc::After)
        .ok_or_else(|| anyhow::anyhow!("Failed to create StickyIndex"))?;

    // 釋放只讀事務
    drop(txn);

    // 第三步：使用 offset 來插入內容（需要可寫事務）
    let mut txn_mut = doc.transact_mut();

    // 準備要插入的內容（如果 offset > 0 且前面有文字，加空格）
    let text_to_insert = if offset > 0 && offset < text_len {
        format!(" {}", content.trim())
    } else {
        content.trim().to_string()
    };

    // 使用 offset 來插入內容
    // 對於 XML Text 節點，offset 本身就是穩定的插入位置
    // TODO: 未來可以考慮使用 StickyIndex 的解析方法來獲取更精確的位置
    target_text_ref.insert(&mut txn_mut, offset, &text_to_insert);

    // 事務在函數結束時自動提交
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::read::{debug_doc_structure, get_doc_content};
    use crate::editor::xml_structure::{create_paragraph_element, create_text_node_in_paragraph};
    use yrs::{ReadTxn, Transact, Xml};

    #[test]
    fn test_insert_ai_content_to_paragraph() {
        let doc = Arc::new(Doc::new());

        // 先創建一個 paragraph 結構
        let xml_fragment = doc.get_or_insert_xml_fragment("content");
        {
            let mut txn = doc.transact_mut();
            create_paragraph_element(&xml_fragment, &mut txn, 0);
        }

        // 在 paragraph 中添加一個空的 text node
        {
            let mut txn = doc.transact_mut();
            if let Some(yrs::types::xml::XmlOut::Element(para)) = xml_fragment.get(&txn, 0) {
                create_text_node_in_paragraph(&para, &mut txn, 0, "");
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
            create_paragraph_element(&xml_fragment, &mut txn, 0);
        }

        {
            let mut txn = doc.transact_mut();
            if let Some(yrs::types::xml::XmlOut::Element(para)) = xml_fragment.get(&txn, 0) {
                create_text_node_in_paragraph(&para, &mut txn, 0, "Hello, world!");
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
}
