use askama::Template;
use crate::session_state::FlashMessage;

#[derive(Template)]
#[template(path = "web/login.html")]
pub struct LoginTemplate {
    pub flash_messages: Vec<FlashMessage>,
}

#[derive(Template)]
#[template(path = "web/admin_dashboard.html")]
pub struct AdminDashboardTemplate {
    pub username: String,
}

#[derive(Template)]
#[template(path = "web/newsletters_form.html")]
pub struct NewslettersFormTemplate {
    pub flash_messages: Vec<FlashMessage>,
    pub idempotency_key: String,
}

#[derive(Template)]
#[template(path = "web/change_password.html")]
pub struct ChangePasswordTemplate {
    pub flash_messages: Vec<FlashMessage>,
}
