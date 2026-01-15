// use yrs::{
//     types::{
//         xml::{XmlFragmentRef, XmlOut, XmlTextRef, XmlElementRef}, 
//         Attrs,
//     },
//     Any,            // Correct type for 0.25 values
//     GetString,      // Trait needed for .get_string()
//     ReadTxn, 
//     TransactionMut, 
//     XmlFragment,    // THIS is the Trait needed for .iter()
//     Text,           // Needed for .format() on XmlTextRef
// };

// /// 1. The Public Function: Bold a word everywhere
// /// Usage: pass the result of doc.get_or_insert_xml_fragment("content")
// pub fn format_all_occurrences(
//     txn: &mut TransactionMut,
//     root: &XmlFragmentRef, // Use the Concrete Reference Type
//     target_word: &str,
// ) {
//     // STEP A: Collect handles (READ PHASE)
//     let mut text_nodes = Vec::new();
    
//     // root implements the XmlFragment TRAIT, so we can pass it
//     collect_text_nodes(txn, root, &mut text_nodes);

//     // STEP B: Mutate handles (WRITE PHASE)
//     for text_node in text_nodes {
//         let content = text_node.get_string(txn);

//         let matches: Vec<(usize, &str)> = content.match_indices(target_word).collect();

//         for (start_idx, _matched_str) in matches {
//             let mut attrs = Attrs::new();
//             // In yrs 0.25+, use Any::from(bool)
//             attrs.insert("bold".into(), Any::from(true));

//             // .format() comes from the yrs::Text trait which XmlTextRef implements
//             text_node.format(
//                 txn,
//                 start_idx as u32,
//                 target_word.len() as u32,
//                 attrs,
//             );
//         }
//     }
// }

// /// 2. The Walker: Recursively find XmlText nodes
// /// We use &impl XmlFragment because both XmlFragmentRef and XmlElementRef implement it
// fn collect_text_nodes(
//     txn: &TransactionMut,
//     parent: &impl XmlFragment, 
//     collector: &mut Vec<XmlTextRef>,
// ) {
//     for child in parent.iter(txn) {
//         match child {
//             XmlOut::Element(elem) => {
//                 // elem is XmlElementRef, which implements XmlFragment trait
//                 collect_text_nodes(txn, &elem, collector);
//             }
//             XmlOut::Text(text) => {
//                 // text is XmlTextRef
//                 collector.push(text);
//             }
//             XmlOut::Fragment(frag) => {
//                 // frag is XmlFragmentRef
//                 collect_text_nodes(txn, &frag, collector);
//             }
//         }
//     }
// }

// /// 3. The "Nuclear Option": Replace Paragraph
// pub fn replace_paragraph_content(
//     txn: &mut TransactionMut,
//     root: &XmlFragmentRef,
//     para_index: usize,
//     new_content: &str,
// ) {
//     let mut current_idx = 0;

//     for child in root.iter(txn) {
//         if let XmlOut::Element(para) = child {
//             if current_idx == para_index {
//                 // 1. Clear existing
//                 let len = para.len(txn);
//                 if len > 0 {
//                     // remove_range comes from XmlFragment trait
//                     para.remove_range(txn, 0, len);
//                 }

//                 // 2. Insert new Text
//                 // push_text returns an XmlTextRef
//                 let xml_text = para.push_text(txn);
                
//                 // push comes from Text trait
//                 xml_text.push(txn, new_content);
//                 return;
//             }
//             current_idx += 1;
//         }
//     }
// }