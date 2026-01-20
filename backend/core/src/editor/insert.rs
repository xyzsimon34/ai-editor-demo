use anyhow::Result;
use std::sync::Arc;
use yrs::{Doc, Text, Transact, XmlFragment};

/// 在 paragraph 中指定 textref 的 sticky point 位置插入 AI 內容
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
        return Ok(()); // 空內容不處理
    }

    let xml_fragment = doc.get_or_insert_xml_fragment("content");
    let mut txn = doc.transact_mut();

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

    // 準備要插入的內容（如果 offset > 0 且前面有文字，加空格）
    let text_to_insert = if offset > 0 && offset < text_len {
        format!(" {}", content.trim())
    } else {
        content.trim().to_string()
    };

    // 在指定位置插入內容
    target_text_ref.insert(&mut txn, offset, &text_to_insert);

    // 事務在函數結束時自動提交，observer 會自動捕獲更新
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
