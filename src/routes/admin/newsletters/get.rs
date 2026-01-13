use axum::response::Html;
use std::fmt::Write;

use crate::session_state::TypedSession;

pub async fn newsletters_form(session: TypedSession) -> Html<String> {
    let mut msg_html = String::new();
    for m in session.get_flash_messages().await {
        writeln!(msg_html, "<p><i>{}</i></p>", m).unwrap();
    }

    let idempotency_key = uuid::Uuid::new_v4();

    Html(format!(
        r#"<!DOCTYPE html>
            <html lang="en">
            <head>
                <meta http-equiv="content-type" content="text/html; charset=utf-8">
                <title>Admin dashboard</title>
            </head>
            <body>
            {msg_html}
            <form action="/admin/newsletters" method="post">
            <label>Title
                <input
                    type="text"
                    placeholder="Enter Title"
                    name="title"
                >
            </label>
            <label>Text Content
                <input
                    type="text"
                    placeholder="Enter Text Content"
                    name="text"
                >
            </label>
            <label>HTML Content
            <input
                type="text"
                placeholder="Enter HTML Content"
                name="html"
            >
        </label>
            <input hidden type="text" name="idempotency_key" value="{idempotency_key}">
            <button type="submit">Submit</button>
        </form>
            </body>
            </html>"#,
    ))
}
