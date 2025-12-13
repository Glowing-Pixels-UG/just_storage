mod delete;
mod download;
mod health;
mod list;
mod search;
mod text_search;
mod upload;

#[cfg(test)]
mod tests;

pub use delete::delete_handler;
pub use download::{download_by_key_handler, download_handler};
pub use health::{health_handler, readiness_handler};
pub use list::list_handler;
pub use search::search_handler;
pub use text_search::text_search_handler;
pub use upload::upload_handler;
