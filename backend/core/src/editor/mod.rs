pub mod insert;
pub mod read;
pub mod write;
pub mod xml_structure;

pub use insert::insert_ai_content_to_paragraph;
pub use read::get_doc_content;
pub use write::{append_ai_content_to_doc, append_ai_content_word_by_word, format_word_stream};
pub use xml_structure::{
    collect_text_nodes, collect_text_nodes_from_elem, create_paragraph_element,
    create_text_node_in_paragraph, debug_doc_structure, get_doc_xml_structure,
    get_last_paragraph_element, get_paragraph_element, get_text_refs_in_paragraph,
    is_field_populated, push_element,
};
