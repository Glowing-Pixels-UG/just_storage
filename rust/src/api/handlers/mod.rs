pub mod api_keys;
pub mod delete;
pub mod download;
pub mod health;
pub mod health_checks;
pub mod list;
pub mod search;
pub mod text_search;
pub mod upload;

#[cfg(test)]
mod tests;

pub use api_keys::{
    create_api_key_handler, delete_api_key_handler, get_api_key_handler, list_api_keys_handler,
    update_api_key_handler,
};
pub use delete::delete_handler;
pub use download::{download_by_key_handler, download_handler};
pub use health::{health_handler, readiness_handler};
pub use list::list_handler;
pub use search::search_handler;
pub use text_search::text_search_handler;
pub use upload::upload_handler;
