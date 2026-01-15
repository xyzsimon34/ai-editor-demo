pub mod read;
pub mod write;

pub use read::get_doc_content;
pub use write::{
    UserWritingState, append_ai_content_to_doc, append_ai_content_word_by_word, prepare_words,
};
