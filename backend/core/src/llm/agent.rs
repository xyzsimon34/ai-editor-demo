use crate::llm::tools::extender;
use anyhow::Result;
use std::sync::Arc;
use yrs::{Doc, GetString, Transact, XmlFragment};

pub async fn new_composer(api_key: &str, role: &str, doc: &Arc<Doc>) -> Result<()> {
    let api_key = api_key.to_string();
    let article_draft = crate::editor::get_doc_content(doc);
    let result = extender::execute_tool(&article_draft, role, &api_key)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to execute tool: {}", e))?;
    println!("result: {}", result);
    let user_state = crate::editor::UserWritingState::new(3);
    // let words = result.char_indices().map(|(_, c)| c.to_string()).collect();
    let words = result.split_whitespace().map(|w| w.to_string()).collect();
    crate::editor::append_ai_content_word_by_word(doc, words, 10, &user_state).await?;
    Ok(())
}

// pub async fn new_linter(api_key: &str, role: &str, doc: Arc<Doc>) -> Result<()> {
//     let api_key = api_key.to_string();

//     // 直接從 XML 讀取 XML 格式內容
//     let xml_content = doc_to_xml_string(doc.clone());

//     let result = extender::execute_tool(&xml_content, role, &api_key)
//         .await
//         .map_err(|e| anyhow::anyhow!("Failed to execute tool: {}", e))?;
//     println!("result: {}", result);
//     replace_text_in_doc(doc, &result);
//     Ok(())
// }

// fn doc_to_plain_text(doc: Arc<Doc>) -> String {
//     let xml_fragment = doc.get_or_insert_xml_fragment("content");
//     let txn = doc.transact();
//     extract_text_from_fragment(&xml_fragment, &txn)
// }

// fn doc_to_xml_string(doc: Arc<Doc>) -> String {
//     let xml_fragment = doc.get_or_insert_xml_fragment("content");
//     let txn = doc.transact();
//     extract_xml_from_fragment(&xml_fragment, &txn)
// }

// // 從 XML Fragment 提取純文字內容（處理所有嵌套結構）
// fn extract_text_from_fragment(
//     fragment: &yrs::types::xml::XmlFragmentRef,
//     txn: &yrs::Transaction,
// ) -> String {
//     let mut content = String::new();
//     let len = fragment.len(txn);

//     for i in 0..len {
//         if let Some(child) = fragment.get(txn, i) {
//             extract_node_content(&child, txn, &mut content, false);
//         }
//     }

//     // 移除末尾多餘的換行
//     content.trim_end_matches('\n').to_string()
// }

// // 遞迴處理節點內容
// fn extract_node_content(
//     node: &yrs::types::xml::XmlOut,
//     txn: &yrs::Transaction,
//     output: &mut String,
//     is_inline: bool,
// ) {
//     match node {
//         yrs::types::xml::XmlOut::Text(text_ref) => {
//             // 使用 GetString trait 的 get_string 方法
//             let text = text_ref.get_string(txn);
//             if !text.is_empty() {
//                 output.push_str(&text);
//             }
//         }
//         yrs::types::xml::XmlOut::Element(elem_ref) => {
//             let tag = elem_ref.tag().as_ref();
//             let elem_len = elem_ref.len(txn);

//             // 判斷是否為區塊級元素（需要換行）
//             let is_block = matches!(
//                 tag,
//                 "paragraph" | "heading" | "code_block" | "blockquote" | "horizontal_rule"
//             );

//             // 判斷是否為換行元素
//             let is_break = matches!(tag, "hard_break" | "br");

//             // 遞迴處理所有子節點
//             for j in 0..elem_len {
//                 if let Some(child) = elem_ref.get(txn, j) {
//                     extract_node_content(&child, txn, output, !is_block);
//                 }
//             }

//             // 根據元素類型添加格式
//             if is_break {
//                 output.push('\n');
//             } else if is_block && !is_inline {
//                 // 區塊級元素結束時添加換行（但不在嵌套的 inline 元素中）
//                 output.push('\n');
//             }
//         }
//         yrs::types::xml::XmlOut::Fragment(fragment_ref) => {
//             // 處理嵌套的 fragment，遞迴提取內容
//             let fragment_len = fragment_ref.len(txn);
//             for i in 0..fragment_len {
//                 if let Some(child) = fragment_ref.get(txn, i) {
//                     extract_node_content(&child, txn, output, is_inline);
//                 }
//             }
//         }
//     }
// }

// // 從 XML Fragment 提取 XML 字符串格式（保留結構）
// fn extract_xml_from_fragment(
//     fragment: &yrs::types::xml::XmlFragmentRef,
//     txn: &yrs::Transaction,
// ) -> String {
//     let mut xml = String::new();
//     let len = fragment.len(txn);

//     for i in 0..len {
//         if let Some(child) = fragment.get(txn, i) {
//             extract_xml_from_node(&child, txn, &mut xml);
//         }
//     }

//     xml
// }

// // 遞迴處理節點，生成 XML 字符串
// fn extract_xml_from_node(
//     node: &yrs::types::xml::XmlOut,
//     txn: &yrs::Transaction,
//     output: &mut String,
// ) {
//     match node {
//         yrs::types::xml::XmlOut::Text(text_ref) => {
//             let text = text_ref.get_string(txn);
//             // 轉義 XML 特殊字符
//             let escaped = text
//                 .replace('&', "&amp;")
//                 .replace('<', "&lt;")
//                 .replace('>', "&gt;")
//                 .replace('"', "&quot;")
//                 .replace('\'', "&apos;");
//             output.push_str(&escaped);
//         }
//         yrs::types::xml::XmlOut::Element(elem_ref) => {
//             let tag = elem_ref.tag().as_ref();
//             let elem_len = elem_ref.len(txn);

//             // 開始標籤
//             output.push('<');
//             output.push_str(tag);
//             output.push('>');

//             // 處理子節點
//             for j in 0..elem_len {
//                 if let Some(child) = elem_ref.get(txn, j) {
//                     extract_xml_from_node(&child, txn, output);
//                 }
//             }

//             // 結束標籤
//             output.push_str("</");
//             output.push_str(tag);
//             output.push('>');
//         }
//         yrs::types::xml::XmlOut::Fragment(fragment_ref) => {
//             // 處理嵌套的 fragment
//             let fragment_len = fragment_ref.len(txn);
//             for i in 0..fragment_len {
//                 if let Some(child) = fragment_ref.get(txn, i) {
//                     extract_xml_from_node(&child, txn, output);
//                 }
//             }
//         }
//     }
// }

// fn write_to_yjs(doc: &Arc<Doc>, text: &str) {
//     insert_text_to_doc(doc, text);
// }

// // fn insert_text_to_doc(doc: &Arc<Doc>, text: &str) {
// //     let xml_fragment = doc.get_or_insert_xml_fragment("content");
// //     let mut txn = doc.transact_mut();
// //     xml_fragment.insert(&mut txn, 0, text);
// //     txn.commit();
// // }

// // fn replace_text_in_doc(doc: Arc<Doc>, text: &str) {
// //     let xml_fragment = doc.get_or_insert_xml_fragment("content");
// //     let mut txn = doc.transact_mut();

// //     // 先刪除所有現有內容
// //     let len = xml_fragment.len(&txn);
// //     if len > 0 {
// //         xml_fragment.remove_range(&mut txn, 0, len);
// //     }

// //     // 然後插入新內容
// //     xml_fragment.insert(&mut txn, 0, text);
// //     txn.commit();
// // }
