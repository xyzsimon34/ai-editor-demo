pub mod read;
pub mod write;

pub use read::get_doc_content;
pub use write::{
    append_ai_content_to_doc, append_ai_content_word_by_word, format_word_stream,
    is_field_populated, push_element,
};
