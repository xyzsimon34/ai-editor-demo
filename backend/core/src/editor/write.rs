use anyhow::Result;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use yrs::{
    Doc, GetString, Text, Transact, Xml, XmlElementRef, XmlFragment, XmlFragmentRef, XmlTextRef,
};

// ============================================================================
// User Writing Detection Context
// ============================================================================

/// 用戶寫入狀態，用於追蹤用戶是否正在寫入
///
/// 當用戶正在輸入時，AI 應該暫停追加內容，避免衝突
#[derive(Clone)]
pub struct UserWritingState {
    /// 標記用戶是否正在寫入
    pub user_writing_flag: Arc<AtomicBool>,
    /// 用戶停止寫入的閾值（毫秒），超過此時間後自動清除標記
    pub writing_timeout_ms: u64,
}

impl UserWritingState {
    /// 創建新的寫入上下文
    ///
    /// # Arguments
    /// * `writing_timeout_ms` - 用戶停止寫入的閾值（毫秒）
    pub fn new(writing_timeout_ms: u64) -> Self {
        Self {
            user_writing_flag: Arc::new(AtomicBool::new(false)),
            writing_timeout_ms,
        }
    }

    /// 檢查用戶是否正在寫入
    ///
    /// # Returns
    /// `true` 如果用戶正在寫入，AI 應該暫停
    /// `false` 如果用戶未在寫入，AI 可以繼續
    pub fn is_user_writing(&self) -> bool {
        self.user_writing_flag.load(Ordering::Relaxed)
    }

    /// 標記用戶開始寫入
    ///
    /// 當收到用戶輸入時調用此方法
    pub fn mark_user_writing(&self) {
        self.user_writing_flag.store(true, Ordering::Relaxed);
    }

    /// 清除用戶寫入標記
    ///
    /// 通常在定時器到期後自動調用
    pub fn clear_user_writing(&self) {
        self.user_writing_flag.store(false, Ordering::Relaxed);
    }
}

// ============================================================================
// Word Preparation
// ============================================================================

/// 將文字預先分割為單詞列表，每個單詞後面會加上空格
/// 最後一個單詞會添加換行符
///
/// # Arguments
/// * `content` - 要處理的文字內容
///
/// # Returns
/// 預處理的單詞列表，一旦中斷即拋棄
///
/// # Example
/// ```
/// let words = format_word_stream("Hello World");
/// // 結果: vec!["Hello ", "World\n"]
/// ```
pub fn format_word_stream(content: &str) -> Vec<String> {
    let words: Vec<&str> = content.split_whitespace().collect();

    if words.is_empty() {
        return Vec::new();
    }

    let mut result: Vec<String> = words
        .iter()
        .take(words.len() - 1)
        .map(|&w| format!("{} ", w))
        .collect();

    if let Some(last) = words.last() {
        result.push(format!("{}\n", last));
    }

    result
}

/// Check if the document has content structure (at least one paragraph)
pub fn is_field_populated(doc: &Arc<Doc>, field_name: &str) -> bool {
    let xml_fragment = doc.get_or_insert_xml_fragment(field_name);
    let txn = doc.transact();
    xml_fragment.len(&txn) > 0
}

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
    // 如果沒有內容，需要等待用戶先創建結構
    if is_field_populated(doc, "content") {
        return Err(anyhow::anyhow!(
            "Document has no content structure yet. User needs to create content first."
        ));
    }

    let xml_fragment = doc.get_or_insert_xml_fragment("content");
    let mut txn = doc.transact_mut();

    // 獲取 fragment 長度
    let len = xml_fragment.len(&txn);

    // 獲取最後一個元素（應該是段落）
    let Some(last_elem) = xml_fragment.get(&txn, len - 1) else {
        return Err(anyhow::anyhow!("Failed to get last element from fragment"));
    };

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

/// 逐字追加預處理的單詞列表到文檔
///
/// **重要**：一旦檢測到用戶寫入，立即停止並拋棄剩餘單詞，不恢復
///
/// # Arguments
/// * `doc` - 共享的 Yrs Doc 實例
/// * `words` - 預處理的單詞列表（Vec<String>），每個單詞已包含空格或換行符
/// * `delay_ms` - 每個單詞之間的延遲（毫秒），用於流式效果，預設100ms
/// * `user_state` - 用戶寫入狀態，用於檢測用戶是否在寫入
///
/// # Returns
/// `Ok(())` 如果成功完成或中斷
/// `Err` 如果發生錯誤
///
/// # Behavior
/// - 每次追加前檢查 `user_state.is_user_writing()`
/// - 如果用戶開始寫入，立即返回 `Ok(())`，拋棄剩餘單詞
/// - 不保留任何狀態，每次調用都是獨立的
pub async fn append_ai_content_word_by_word(
    doc: &Arc<Doc>,
    words: Vec<String>,
    delay_ms: u64,
    user_state: &UserWritingState,
) -> Result<()> {
    if words.is_empty() {
        return Ok(());
    }

    // 在開始前檢查一次
    if user_state.is_user_writing() {
        tracing::info!("User is writing, skipping AI append");
        return Ok(()); // 直接拋棄所有單詞
    }

    // 遍歷預處理的單詞列表
    for word in words {
        // 每次追加前再次檢查用戶是否開始寫入
        if user_state.is_user_writing() {
            tracing::info!(
                "User started writing, stopping AI append and discarding remaining words"
            );
            return Ok(()); // 立即停止，拋棄剩餘單詞
        }

        // 追加單詞（已包含空格或換行符）
        append_ai_content_to_doc(doc, &word)?;

        // 延遲以產生流式效果
        if delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }
    }

    Ok(())
}

/// Apply text replacements to all text nodes in the document
///
/// This function traverses the XML fragment, finds all text nodes,
/// and applies the given replacements to each text node.
///
/// # Arguments
/// * `doc` - Shared Yrs Doc instance
/// * `field_name` - Field name of the XML fragment (usually "content")
/// * `replacements` - Vector of replacement rules
///
/// # Returns
/// `Ok(())` if successful, `Err` if failed
pub fn apply_replacements(
    doc: &Arc<Doc>,
    field_name: &str,
    replacements: &[crate::llm::tools::emoji_replacer::Replacement],
) -> Result<()> {
    if replacements.is_empty() {
        return Ok(());
    }

    let xml_fragment = doc.get_or_insert_xml_fragment(field_name);

    // CRITICAL: XmlTextRef references are tied to the transaction they were created in.
    // We MUST collect them within the write transaction, not before it.
    let mut txn = doc.transact_mut();
    let mut text_nodes = Vec::new();
    collect_text_nodes(&txn, &xml_fragment, &mut text_nodes);

    // Apply replacements to each text node
    for text_ref in text_nodes {
        let current_text = text_ref.get_string(&txn);
        let mut new_text = current_text.clone();

        // Apply all replacements
        for replacement in replacements {
            if !replacement.replace.is_empty() {
                new_text = new_text.replace(&replacement.replace, &replacement.with);
            }
        }

        // Only update if text changed
        if new_text != current_text {
            let len = text_ref.len(&txn);
            if len > 0 {
                // Remove all existing text
                text_ref.remove_range(&mut txn, 0, len);
            }
            // Insert new text
            text_ref.insert(&mut txn, 0, &new_text);
            tracing::debug!("Applied replacement: '{}' -> '{}'", current_text, new_text);
        }
    }

    // Transaction commits here when it goes out of scope
    // This triggers the observer in mono.rs to broadcast the update
    Ok(())
}

/// Helper: Recursively find all XmlTextRef nodes in a fragment
/// Uses ReadTxn trait so it works with both Transaction and TransactionMut
fn collect_text_nodes(
    txn: &impl yrs::ReadTxn,
    fragment: &yrs::XmlFragmentRef,
    collector: &mut Vec<yrs::XmlTextRef>,
) {
    use yrs::types::xml::XmlOut;

    let len = fragment.len(txn);
    for i in 0..len {
        if let Some(child) = fragment.get(txn, i) {
            match child {
                XmlOut::Element(elem) => {
                    // Recurse into element
                    collect_text_nodes_from_elem(txn, &elem, collector);
                }
                XmlOut::Text(text_ref) => {
                    collector.push(text_ref);
                }
                _ => {}
            }
        }
    }
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
pub fn append_element_into_element(
    doc: &Arc<Doc>,
    field_name: &str,
    element_name: &str,
    content: &str,
    attrs: &[(&str, &str)],
) -> Result<()> {
    let xml_fragment = doc.get_or_insert_xml_fragment(field_name);

    let mut txn = doc.transact_mut();
    let len = xml_fragment.len(&txn);
    let parent_elem = xml_fragment.get(&txn, len - 1);

    let Some(yrs::types::xml::XmlOut::Element(parent_elem)) = parent_elem else {
        return Err(anyhow::anyhow!("Last element is not an Element"));
    };

    let elem_prelim = yrs::types::xml::XmlElementPrelim::empty(element_name);

    let len = parent_elem.len(&txn);
    let elem = parent_elem.insert(&mut txn, len, elem_prelim);

    let text_prelim = yrs::XmlTextPrelim::new(content);

    elem.insert(&mut txn, 0, text_prelim);

    for (key, value) in attrs {
        elem.insert_attribute(&mut txn, *key, *value);
    }
    return Ok(());
}
#[cfg(test)]
mod tests {
    use super::*;
    use yrs::{XmlOut, XmlTextPrelim};

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

    #[test]
    fn test_format_word_stream() {
        let words = format_word_stream("Hello World");
        assert_eq!(words, vec!["Hello ", "World\n"]);

        let words2 = format_word_stream("Single");
        assert_eq!(words2, vec!["Single\n"]);

        let words3 = format_word_stream("  Multiple   Words   Here  ");
        assert_eq!(words3, vec!["Multiple ", "Words ", "Here\n"]);

        let words_with_dot = format_word_stream("Hello. World.");
        assert_eq!(words_with_dot, vec!["Hello. ", "World.\n"]);

        let words_with_comma = format_word_stream("Hello, World.");
        assert_eq!(words_with_comma, vec!["Hello, ", "World.\n"]);

        let words_with_question_mark = format_word_stream("Hello World?");
        assert_eq!(words_with_question_mark, vec!["Hello ", "World?\n"]);

        let words_with_exclamation_mark = format_word_stream("Hello World!");
        assert_eq!(words_with_exclamation_mark, vec!["Hello ", "World!\n"]);

        let words_with_colon = format_word_stream("Hello: World:");
        assert_eq!(words_with_colon, vec!["Hello: ", "World:\n"]);

        let empty = format_word_stream("");
        assert!(empty.is_empty());

        let whitespace = format_word_stream("   ");
        assert!(whitespace.is_empty());
    }

    #[tokio::test]
    async fn test_append_word_by_word_with_user_interruption() {
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

        let user_state = UserWritingState::new(2000);
        let words = format_word_stream("Hello World");

        // 開始追加
        let doc_clone = doc.clone();
        let user_state_clone = user_state.clone();
        let append_task = tokio::spawn(async move {
            append_ai_content_word_by_word(&doc_clone, words, 50, &user_state_clone).await
        });

        // 模擬用戶開始寫入（在第一個單詞後）
        tokio::time::sleep(Duration::from_millis(60)).await;
        user_state.mark_user_writing();

        // 等待追加任務完成
        let result = append_task.await.unwrap();
        assert!(result.is_ok());

        // 驗證只有部分內容被追加（因為用戶中斷）
        let content = crate::editor::read::get_doc_content(&doc);
        assert!(content.contains("Existing"));
        // 可能只有 "Hello " 被追加，或者都沒有，取決於時機
    }

    #[tokio::test]
    async fn test_append_word_by_word_complete() {
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

        let user_state = UserWritingState::new(2000);
        let words = format_word_stream("Test Word");

        // 完整追加（用戶未中斷）
        let result = append_ai_content_word_by_word(&doc, words, 10, &user_state).await;
        assert!(result.is_ok());

        let content = crate::editor::read::get_doc_content(&doc);
        assert!(content.contains("Existing"));
        assert!(content.contains("Test"));
        assert!(content.contains("Word"));
    }

    #[tokio::test]
    async fn test_append_word_by_word_skips_when_user_writing() {
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

        let user_state = UserWritingState::new(2000);

        // 標記用戶正在寫入
        user_state.mark_user_writing();

        let words = format_word_stream("Should Not Append");

        // 嘗試追加，但應該被跳過
        let result = append_ai_content_word_by_word(&doc, words, 10, &user_state).await;
        assert!(result.is_ok()); // 返回 Ok，但沒有追加內容

        let content = crate::editor::read::get_doc_content(&doc);
        assert_eq!(content, "Existing"); // 內容未改變
    }

    #[tokio::test] // 如果是 async 函數，需要配合 test runtime
    async fn test_append_element_into_element() {
        let doc = Arc::new(Doc::new());

        let fragment = doc.get_or_insert_xml_fragment("content");

        // 創建段落結構
        {
            let mut txn = doc.transact_mut();
            let para = yrs::types::xml::XmlElementPrelim::empty("paragraph");
            fragment.insert(&mut txn, 0, para);
        }

        append_element_into_element(
            &doc,
            "content",
            "paragraph",
            "Hello, world!",
            &[("id", "123")],
        )
        .unwrap();

        let txn = doc.transact();
        let len = fragment.len(&txn);

        let Some(elem) = fragment.get(&txn, len - 1) else {
            panic!("Failed to get last element from fragment");
        };

        let yrs::types::xml::XmlOut::Element(para) = elem else {
            panic!("Last element is not an Element");
        };

        let Some(target_elem) = para.get(&txn, 0) else {
            panic!("Failed to get first child from paragraph");
        };

        let yrs::types::xml::XmlOut::Element(tar) = target_elem else {
            panic!("First child is not an element");
        };

        let Some(text) = tar.get(&txn, 0) else {
            panic!("Failed to get text node");
        };

        let yrs::types::xml::XmlOut::Text(src_text_ref) = text else {
            panic!("Text node is not a text node");
        };

        let src_attrs = tar.get_attribute(&txn, "id");
        let attr_value = src_attrs.map(|v| v.to_string(&txn));

        assert_eq!(attr_value, Some("123".to_string()));

        let src_text = src_text_ref.get_string(&txn);
        assert_eq!(src_text, "Hello, world!");
    }
}
