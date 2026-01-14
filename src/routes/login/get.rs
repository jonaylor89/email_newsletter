use askama::Template;
use axum::response::Html;

use crate::session_state::TypedSession;
use crate::web_templates::LoginTemplate;

pub async fn login_form(session: TypedSession) -> Html<String> {
    let flash_messages = session.get_flash_messages().await;

    let template = LoginTemplate { flash_messages };

    Html(template.render().unwrap())
}
