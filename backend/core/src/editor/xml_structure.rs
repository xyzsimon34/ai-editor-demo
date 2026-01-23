use anyhow::Result;
use std::sync::Arc;
use yrs::{
    Doc, GetString, ReadTxn, Transact, TransactionMut, Xml, XmlElementPrelim, XmlElementRef,
    XmlFragment, XmlOut, XmlTextPrelim, XmlTextRef,
};

// ============================================================================
// Text Node Collection
// ============================================================================

/// 遞迴收集 XML Fragment 中的所有文字節點
///
/// # Arguments
/// * `txn` - 只讀事務（支援 Transaction 或 TransactionMut）
/// * `fragment` - XML Fragment 引用
/// * `collector` - 用於收集文字節點的向量
pub fn collect_text_nodes(
    txn: &impl ReadTxn,
    fragment: &yrs::XmlFragmentRef,
    collector: &mut Vec<XmlTextRef>,
) {
    use yrs::types::xml::XmlOut;

    let len = fragment.len(txn);
    for i in 0..len {
        if let Some(child) = fragment.get(txn, i) {
            match child {
                XmlOut::Element(elem) => {
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

/// 遞迴收集 XML Element 中的所有文字節點
///
/// # Arguments
/// * `txn` - 只讀事務（支援 Transaction 或 TransactionMut）
/// * `elem` - XML Element 引用
/// * `collector` - 用於收集文字節點的向量
pub fn collect_text_nodes_from_elem(
    txn: &impl ReadTxn,
    elem: &XmlElementRef,
    collector: &mut Vec<XmlTextRef>,
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
// Paragraph Element Operations
// ============================================================================

/// 從 XML Fragment 中獲取指定索引的 paragraph 元素
///
/// # Arguments
/// * `fragment` - XML Fragment 引用
/// * `txn` - 只讀事務
/// * `paragraph_index` - paragraph 在 fragment 中的索引
///
/// # Returns
/// `Ok(XmlElementRef)` 如果成功找到 paragraph，`Err` 如果失敗
pub fn get_paragraph_element(
    fragment: &yrs::XmlFragmentRef,
    txn: &impl ReadTxn,
    paragraph_index: u32,
) -> Result<XmlElementRef> {
    let Some(child) = fragment.get(txn, paragraph_index) else {
        return Err(anyhow::anyhow!("No element at index {}", paragraph_index));
    };

    let XmlOut::Element(para) = child else {
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

    Ok(para)
}

/// 獲取 paragraph 中的所有文字節點引用
///
/// # Arguments
/// * `doc` - 共享的 Yrs Doc 實例
/// * `paragraph_index` - paragraph 在 fragment 中的索引
///
/// # Returns
/// `Ok(Vec<XmlTextRef>)` 如果成功，`Err` 如果失敗
pub fn get_text_refs_in_paragraph(
    doc: &Arc<Doc>,
    paragraph_index: u32,
) -> Result<Vec<XmlTextRef>> {
    let xml_fragment = doc.get_or_insert_xml_fragment("content");
    let txn = doc.transact();

    let para = get_paragraph_element(&xml_fragment, &txn, paragraph_index)?;

    let mut text_refs = Vec::new();
    collect_text_nodes_from_elem(&txn, &para, &mut text_refs);
    Ok(text_refs)
}

/// 獲取 XML Fragment 中的最後一個 paragraph 元素
///
/// # Arguments
/// * `fragment` - XML Fragment 引用
/// * `txn` - 只讀事務
///
/// # Returns
/// `Ok(XmlElementRef)` 如果成功找到 paragraph，`Err` 如果失敗
pub fn get_last_paragraph_element(
    fragment: &yrs::XmlFragmentRef,
    txn: &impl ReadTxn,
) -> Result<XmlElementRef> {
    let len = fragment.len(txn);
    if len == 0 {
        return Err(anyhow::anyhow!("Fragment is empty"));
    }

    get_paragraph_element(fragment, txn, (len - 1) as u32)
}

// ============================================================================
// Element Creation and Manipulation
// ============================================================================

/// 創建一個新的 paragraph 元素並插入到 fragment 中
///
/// # Arguments
/// * `fragment` - XML Fragment 引用
/// * `txn` - 可寫事務
/// * `index` - 插入位置索引
///
/// # Returns
/// 創建的 paragraph 元素引用
pub fn create_paragraph_element(
    fragment: &yrs::XmlFragmentRef,
    txn: &mut TransactionMut<'_>,
    index: u32,
) -> XmlElementRef {
    let para = XmlElementPrelim::empty("paragraph");
    fragment.insert(txn, index, para)
}

/// 在 paragraph 元素中創建一個文字節點
///
/// # Arguments
/// * `para` - paragraph 元素引用
/// * `txn` - 可寫事務
/// * `index` - 插入位置索引
/// * `content` - 文字內容
///
/// # Returns
/// 創建的文字節點引用
pub fn create_text_node_in_paragraph(
    para: &XmlElementRef,
    txn: &mut TransactionMut<'_>,
    index: u32,
    content: &str,
) -> XmlTextRef {
    para.insert(txn, index, XmlTextPrelim::new(content))
}

/// 推送一個新元素到父元素或 fragment 中
///
/// # Arguments
/// * `parent` - 父元素或 fragment（XmlOut）
/// * `new_tag_name` - 新元素的標籤名稱
/// * `content` - 元素內容
/// * `attrs` - 元素屬性（鍵值對）
/// * `txn` - 可寫事務
///
/// # Returns
/// `Ok(XmlElementRef)` 如果成功，`Err` 如果失敗
pub fn push_element(
    parent: &XmlOut,
    new_tag_name: &str,
    content: &str,
    attrs: &[(&str, &str)],
    txn: &mut TransactionMut<'_>,
) -> Result<XmlElementRef> {
    let new_elem = match parent {
        XmlOut::Element(elem) => {
            let len = elem.len(txn);
            elem.insert(txn, len, XmlElementPrelim::empty(new_tag_name))
        }
        XmlOut::Fragment(fragment) => {
            let len = fragment.len(txn);
            fragment.insert(txn, len, XmlElementPrelim::empty(new_tag_name))
        }
        _ => return Err(anyhow::anyhow!("Parent is not an element or fragment")),
    };

    for (key, value) in attrs {
        new_elem.insert_attribute(txn, *key, *value);
    }

    new_elem.insert(txn, 0, XmlTextPrelim::new(content));

    Ok(new_elem)
}

// ============================================================================
// Document Structure Queries
// ============================================================================

/// 獲取文檔的 XML 結構
///
/// # Arguments
/// * `doc` - 共享的 Yrs Doc 實例
///
/// # Returns
/// XML 結構的 XmlOut 表示
pub fn get_doc_xml_structure(doc: &Arc<Doc>) -> XmlOut {
    let xml_fragment = doc.get_or_insert_xml_fragment("content");
    XmlOut::Fragment(xml_fragment)
}

/// Debug 整個 doc 的結構
///
/// 返回一個可以用於 debug 打印的字符串表示
///
/// # Arguments
/// * `doc` - 共享的 Yrs Doc 實例
///
/// # Returns
/// 結構化的字符串表示
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
                XmlOut::Text(text_node) => {
                    let text = text_node.get_string(&txn);
                    result.push_str(&format!("Text({} chars): \"{}\"\n", text.len(), text));
                }
                XmlOut::Element(elem) => {
                    let tag = elem.tag().as_ref();
                    let child_count = elem.len(&txn);
                    result.push_str(&format!("Element<{}> ({} children)\n", tag, child_count));

                    // 遞迴打印子節點
                    for j in 0..child_count {
                        if let Some(child) = elem.get(&txn, j) {
                            match &child {
                                XmlOut::Text(text_node) => {
                                    let text = text_node.get_string(&txn);
                                    result.push_str(&format!(
                                        "    [{}]: Text({} chars): \"{}\"\n",
                                        j,
                                        text.len(),
                                        text
                                    ));
                                }
                                XmlOut::Element(child_elem) => {
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
                XmlOut::Fragment(_) => {
                    result.push_str("Fragment\n");
                }
            }
        }
    }

    result
}

// ============================================================================
// Field Validation
// ============================================================================

/// 檢查文檔欄位是否已填充（至少有一個元素）
///
/// # Arguments
/// * `doc` - 共享的 Yrs Doc 實例
/// * `field_name` - 欄位名稱（通常是 "content"）
///
/// # Returns
/// `true` 如果欄位已填充，`false` 如果為空
pub fn is_field_populated(doc: &Arc<Doc>, field_name: &str) -> bool {
    let xml_fragment = doc.get_or_insert_xml_fragment(field_name);
    let txn = doc.transact();
    xml_fragment.len(&txn) > 0
}
