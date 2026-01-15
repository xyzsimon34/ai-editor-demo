pub mod read;
pub mod write;

pub use read::get_doc_content;
pub use write::{append_ai_content_to_doc, has_content_structure};
