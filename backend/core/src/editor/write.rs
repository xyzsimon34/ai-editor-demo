use anyhow::Result;
use std::sync::Arc;
use yrs::{Doc, Text, Transact, XmlFragment};

/// 將 AI 生成的內容寫入 Doc 的最後一個段落
///
/// # Arguments
/// * `doc` - 共享的 Yrs Doc 實例
/// * `content` - 要寫入的文字內容
///
/// # Returns
/// `Ok(())` 如果成功，`Err` 如果失敗
///
/// # Errors
/// - 如果文檔還沒有內容結構（用戶尚未創建內容）
/// - 如果最後一個元素不是段落
/// - 如果段落為空或沒有文字節點
///
/// # Example
/// ```rust
/// use std::sync::Arc;
/// use yrs::Doc;
/// use backend_core::editor::append_ai_content_to_doc;
///
/// let doc = Arc::new(Doc::new());
/// // ... 用戶先創建內容結構 ...
/// append_ai_content_to_doc(&doc, "AI generated text")?;
/// ```
pub fn append_ai_content_to_doc(doc: &Arc<Doc>, content: &str) -> Result<()> {
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

    // 獲取最後一個元素（應該是段落）
    let Some(last_elem) = xml_fragment.get(&txn, len - 1) else {
        return Err(anyhow::anyhow!("Failed to get last element from fragment"));
    };

    // 檢查是否為段落元素
    let yrs::types::xml::XmlOut::Element(para) = last_elem else {
        return Err(anyhow::anyhow!("Last element is not an Element"));
    };

    // 檢查標籤是否為 paragraph
    if para.tag().as_ref() != "paragraph" {
        return Err(anyhow::anyhow!(
            "Last element is not a paragraph (tag: {})",
            para.tag().as_ref()
        ));
    }

    // 獲取段落長度
    let para_len = para.len(&txn);
    if para_len == 0 {
        return Err(anyhow::anyhow!("Paragraph is empty, cannot append text"));
    }

    // 獲取最後一個子節點（應該是文字節點）
    let Some(yrs::types::xml::XmlOut::Text(text_ref)) = para.get(&txn, para_len - 1) else {
        return Err(anyhow::anyhow!("Last child is not a text node"));
    };

    // 在文字末尾插入 AI 生成的內容
    let current_len = text_ref.len(&txn);
    // 如果已有文字，在前面加空格
    let text_to_insert = if current_len > 0 {
        format!(" {}", content.trim())
    } else {
        content.trim().to_string()
    };

    text_ref.insert(&mut txn, current_len, &text_to_insert);

    // 事務在函數結束時自動提交，observer 會自動捕獲更新
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use yrs::XmlTextPrelim;

    #[test]
    fn test_append_ai_content_to_empty_doc() {
        let doc = Arc::new(Doc::new());
        let result = append_ai_content_to_doc(&doc, "test");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Document has no content structure")
        );
    }

    #[test]
    fn test_append_ai_content_to_doc_with_paragraph() {
        let doc = Arc::new(Doc::new());
        let fragment = doc.get_or_insert_xml_fragment("content");

        // 先創建一個段落結構
        {
            let mut txn = doc.transact_mut();
            let para = yrs::types::xml::XmlElementPrelim::empty("paragraph");
            fragment.insert(&mut txn, 0, para);
        }

        // 在段落中添加文字節點
        {
            let mut txn = doc.transact_mut();
            if let Some(yrs::types::xml::XmlOut::Element(para)) = fragment.get(&txn, 0) {
                para.insert(&mut txn, 0, XmlTextPrelim::new("Existing text"));
            }
        }

        // 現在可以追加 AI 內容
        let result = append_ai_content_to_doc(&doc, "AI content");
        assert!(result.is_ok());

        // 驗證內容已添加
        let content = crate::editor::read::get_doc_content(&doc);
        assert!(content.contains("Existing text"));
        assert!(content.contains("AI content"));
    }

    #[test]
    fn test_append_empty_content() {
        let doc = Arc::new(Doc::new());
        let fragment = doc.get_or_insert_xml_fragment("content");

        // 創建段落結構
        {
            let mut txn = doc.transact_mut();
            let para = yrs::types::xml::XmlElementPrelim::empty("paragraph");
            fragment.insert(&mut txn, 0, para);
        }

        {
            let mut txn = doc.transact_mut();
            if let Some(yrs::types::xml::XmlOut::Element(para)) = fragment.get(&txn, 0) {
                para.insert(&mut txn, 0, XmlTextPrelim::new("Existing"));
            }
        }

        // 空內容應該被忽略
        let result = append_ai_content_to_doc(&doc, "   ");
        assert!(result.is_ok());

        let content = crate::editor::read::get_doc_content(&doc);
        assert_eq!(content, "Existing");
    }
}
