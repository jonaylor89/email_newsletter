mod dashboard;
mod logout;
mod newsletters;
mod password;

pub use dashboard::{admin_dashboard, get_username};
pub use logout::log_out;
pub use newsletters::{newsletters_form, publish_newsletter};
pub use password::{change_password, change_password_form};
