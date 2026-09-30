//! Modal dialogs and overlay menus (Preferences, Search, Rename, Delete, Confirm).

pub mod confirm;
pub mod delete;
pub mod rename;
pub mod search;

#[allow(unused_imports)]
pub use confirm::{render_confirm_modal, ConfirmModalAction};
#[allow(unused_imports)]
pub use delete::{render_delete_confirm_modal, DeleteModalAction};
#[allow(unused_imports)]
pub use rename::{render_rename_modal, RenameModalAction};
#[allow(unused_imports)]
pub use search::{render_search_modal, SearchModalAction};
