use askama::Template;
use axum::response::Html;

use crate::session_state::TypedSession;
use crate::web_templates::ChangePasswordTemplate;

pub async fn change_password_form(session: TypedSession) -> Html<String> {
    let flash_messages = session.get_flash_messages().await;

    let template = ChangePasswordTemplate { flash_messages };

    Html(template.render().unwrap())
}
