use crate::{http, opts::*};

use std::{sync::Arc, time::Duration};

use atb_cli_utils::AtbCli;
use backend_core::{sqlx_postgres, temporal};
use tokio::sync::broadcast;
use yrs::{Doc, GetString, Text, Transact, XmlFragment, XmlTextPrelim};

// 從 XML Fragment 提取純文字內容（處理所有嵌套結構）
fn extract_text_from_fragment(
    fragment: &yrs::types::xml::XmlFragmentRef,
    txn: &yrs::Transaction,
) -> String {
    let mut content = String::new();
    let len = fragment.len(txn);

    for i in 0..len {
        if let Some(child) = fragment.get(txn, i) {
            extract_node_content(&child, txn, &mut content, false);
        }
    }

    // 移除末尾多餘的換行
    content.trim_end_matches('\n').to_string()
}

// 遞迴處理節點內容
fn extract_node_content(
    node: &yrs::types::xml::XmlOut,
    txn: &yrs::Transaction,
    output: &mut String,
    is_inline: bool,
) {
    match node {
        yrs::types::xml::XmlOut::Text(text_ref) => {
            // 使用 GetString trait 的 get_string 方法
            let text = text_ref.get_string(txn);
            if !text.is_empty() {
                output.push_str(&text);
            }
        }
        yrs::types::xml::XmlOut::Element(elem_ref) => {
            let tag = elem_ref.tag().as_ref();
            let elem_len = elem_ref.len(txn);

            // 判斷是否為區塊級元素（需要換行）
            let is_block = matches!(
                tag,
                "paragraph" | "heading" | "code_block" | "blockquote" | "horizontal_rule"
            );

            // 判斷是否為換行元素
            let is_break = matches!(tag, "hard_break" | "br");

            // 遞迴處理所有子節點
            for j in 0..elem_len {
                if let Some(child) = elem_ref.get(txn, j) {
                    extract_node_content(&child, txn, output, !is_block);
                }
            }

            // 根據元素類型添加格式
            if is_break {
                output.push('\n');
            } else if is_block && !is_inline {
                // 區塊級元素結束時添加換行（但不在嵌套的 inline 元素中）
                output.push('\n');
            }
        }
        yrs::types::xml::XmlOut::Fragment(fragment_ref) => {
            // 處理嵌套的 fragment，遞迴提取內容
            let fragment_len = fragment_ref.len(txn);
            for i in 0..fragment_len {
                if let Some(child) = fragment_ref.get(txn, i) {
                    extract_node_content(&child, txn, output, is_inline);
                }
            }
        }
    }
}

// 讓 AI agent 從 Doc 獲取純文字內容
#[allow(dead_code)]
fn get_plain_text_from_doc(doc: &Doc) -> String {
    let xml_fragment = doc.get_or_insert_xml_fragment("content");
    let txn = doc.transact();
    extract_text_from_fragment(&xml_fragment, &txn)
}

pub async fn run(
    db_opts: DatabaseOpts,
    http_opts: HttpOpts,
    worker_opts: WorkerOpts,
    opts: Opts,
) -> anyhow::Result<()> {
    let client_id = crate::Cli::client_id();
    let pg_pool = sqlx_postgres::connect_pg(&db_opts.postgres, 30, Some(&client_id)).await?;
    let client = temporal::try_connect_temporal(
        &worker_opts.temporal.temporal,
        &worker_opts.temporal.namespace,
        Duration::from_secs(30),
    )
    .await?;
    let http_client = client.clone();

    let task_queue = worker_opts.temporal.task_queue.clone();
    let worker_config = crate::worker::worker_config(&worker_opts)?;
    let worker_handle =
        std::thread::spawn(move || crate::worker::start_worker(client, worker_config));

    // Initialize the Yrs Document for collaborative editing
    // Start with empty fragment - y-prosemirror will handle structure automatically
    let doc = Arc::new(Doc::new());
    let _xml_fragment = doc.get_or_insert_xml_fragment("content");

    // Create Broadcast Channel (Server -> All Clients)
    let (broadcast_tx, _) = broadcast::channel(100);

    // Setup Observer: When Yrs changes (by User OR AI), broadcast the delta
    let tx_clone = broadcast_tx.clone();
    let _sub = doc.observe_update_v1(move |_txn, update_event| {
        let update = update_event.update.to_vec();
        // Send binary update to all connected clients
        let _ = tx_clone.send(update);
    });

    // Spawn "The AI Agent" (Ghost Writer Demo)
    let ai_doc = doc.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;
            tracing::info!("🤖 AI is writing...");

            // 先讀取當前文檔內容
            let current_content = get_plain_text_from_doc(&ai_doc);

            if !current_content.is_empty() {
                tracing::info!("📄 Current document content: {}", current_content);
            }

            // 然後在同一個可寫事務中進行寫入操作
            let xml_fragment = ai_doc.get_or_insert_xml_fragment("content");
            let mut txn = ai_doc.transact_mut();

            // Wait for user to create content first (paragraph structure)
            // Then append AI text to the last paragraph
            let len = xml_fragment.len(&txn);
            if len == 0 {
                tracing::info!("⏳ Waiting for user to create content first...");
                continue;
            }

            // Get the last element (should be a paragraph)
            let Some(last_elem) = xml_fragment.get(&txn, len - 1) else {
                continue;
            };

            // Check if it's a paragraph element
            let yrs::types::xml::XmlOut::Element(para) = last_elem else {
                continue;
            };

            // Get the paragraph's tag name
            if para.tag().as_ref() != "paragraph" {
                continue;
            }

            // Try to find a text node in the paragraph and append to it
            let para_len = para.len(&txn);
            if para_len == 0 {
                tracing::info!("⚠️ Empty paragraph, would create text node");
                continue;
            }

            // Check the last child - if it's text, append to it
            let Some(yrs::types::xml::XmlOut::Text(text_ref)) = para.get(&txn, para_len - 1) else {
                tracing::info!("⚠️ Last child is not text, would create new text node");
                continue;
            };

            // Insert text at the end
            let current_len = text_ref.len(&txn);
            text_ref.insert(&mut txn, current_len, " [AI was here] ");
            tracing::info!("✅ AI appended text!");

            // The observer above automatically catches this and updates the frontend!
        }
    });

    http::start_http(
        pg_pool,
        http_client,
        http_opts,
        task_queue,
        opts.openai_api_key,
        doc,
        broadcast_tx,
    )
    .await?;

    worker_handle
        .join()
        .map_err(|e| anyhow::anyhow!("worker thread panicked: {:?}", e))??;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_plain_text_from_doc() {
        let doc = Doc::new();
        let text = get_plain_text_from_doc(&doc);
        assert_eq!(text, "");
    }

    #[test]
    fn test_get_plain_text_from_doc_with_data() {
        let doc = Doc::new();
        let f = doc.get_or_insert_xml_fragment("content");

        // 使用 block scope 確保可寫事務在讀取前結束
        {
            let mut txn = doc.transact_mut();
            f.insert(&mut txn, 0, XmlTextPrelim::new("hello, world!"));
        } // txn 在這裡結束

        // 現在可以安全地創建只讀事務
        let text = get_plain_text_from_doc(&doc);
        assert_eq!(text, "hello, world!");
    }

    #[test]
    fn test_extract_text_from_fragment() {
        let doc = Doc::new();
        let xml_fragment = doc.get_or_insert_xml_fragment("content");
        let txn = doc.transact();
        let text = extract_text_from_fragment(&xml_fragment, &txn);
        assert_eq!(text, "");
    }

    #[test]
    fn test_extract_text_from_fragment_with_data() {
        let doc = Doc::new();
        let f = doc.get_or_insert_xml_fragment("content");

        // 使用可寫事務插入內容
        {
            let mut txn = doc.transact_mut();
            f.insert(&mut txn, 0, XmlTextPrelim::new("hello, world!"));
        }

        // 使用只讀事務提取文字
        let txn = doc.transact();
        let text = extract_text_from_fragment(&f, &txn);
        assert_eq!(text, "hello, world!");
    }
}
