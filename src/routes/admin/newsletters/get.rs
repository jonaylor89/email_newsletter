use askama::Template;
use axum::response::Html;

use crate::session_state::TypedSession;
use crate::web_templates::NewslettersFormTemplate;

pub async fn newsletters_form(session: TypedSession) -> Html<String> {
    let flash_messages = session.get_flash_messages().await;
    let idempotency_key = uuid::Uuid::new_v4().to_string();

    let template = NewslettersFormTemplate {
        flash_messages,
        idempotency_key,
    };

    Html(template.render().unwrap())
}
