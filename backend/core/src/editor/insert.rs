use anyhow::Result;
use std::sync::Arc;
use yrs::{Doc, Text, Transact, XmlFragment};

/// 在用戶游標停頓位置插入 AI 生成的內容
///
/// 使用 StickyIndex 來追蹤游標位置，即使文檔結構發生變化也能正確插入
///
/// # Arguments
/// * `doc` - 共享的 Yrs Doc 實例
/// * `text` - 文字節點引用
/// * `user_cursor_pos` - 用戶游標在該文字節點中的位置（字符索引）
/// * `content` - 要插入的 AI 生成內容
///
/// # Returns
/// `Ok(())` 如果成功，`Err` 如果失敗
///
/// # Example
/// ```rust

pub fn insert_ai_content_at_index(doc: &Arc<Doc>, index: u32, content: &str) -> Result<()> {
    if content.trim().is_empty() {
        return Ok(()); // 空內容不處理
    }

    let xml_fragment = doc.get_or_insert_xml_fragment("content");
    let mut txn = doc.transact_mut();

    // 獲取 fragment 長度
    let len = xml_fragment.len(&txn);

    // 如果沒有內容，需要等待用戶先創建結構
    if len == 0 {
        return Err(anyhow::anyhow!(
            "Document has no content structure yet. User needs to create content first."
        ));
    }

    // 找到包含該 index 的文字節點和相對位置
    let mut current_pos = 0u32;
    let mut target_text_ref: Option<yrs::types::xml::XmlTextRef> = None;
    let mut target_offset = 0u32;

    for i in 0..len {
        if let Some(yrs::types::xml::XmlOut::Element(para)) = xml_fragment.get(&txn, i) {
            if para.tag().as_ref() == "paragraph" {
                let para_len = para.len(&txn);
                for j in 0..para_len {
                    if let Some(yrs::types::xml::XmlOut::Text(text_ref)) = para.get(&txn, j) {
                        let text_len = text_ref.len(&txn) as u32;
                        if current_pos <= index && index <= current_pos + text_len {
                            target_text_ref = Some(text_ref.clone());
                            target_offset = index - current_pos;
                            break;
                        }
                        current_pos += text_len;
                    }
                }
                if target_text_ref.is_some() {
                    break;
                }
            }
        }
    }

    let (text_ref, insert_pos) = match target_text_ref {
        Some(text) => (text, target_offset),
        None => return crate::editor::write::append_ai_content_to_doc(doc, content),
    };

    // 在該位置插入內容
    let text_to_insert = if insert_pos > 0 {
        format!(" {}", content.trim())
    } else {
        content.trim().to_string()
    };
    text_ref.insert(&mut txn, insert_pos, &text_to_insert);

    // 事務在函數結束時自動提交，observer 會自動捕獲更新
    Ok(())
}
