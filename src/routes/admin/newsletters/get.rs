use actix_web::{http::header::ContentType, HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;

pub async fn newsletters_form(
    flash_messages: IncomingFlashMessages,
) -> Result<HttpResponse, actix_web::Error> {
    let mut msg_html = String::new();
    for m in flash_messages.iter() {
        writeln!(msg_html, "<p><i>{}</i></p>", m.content()).unwrap();
    }

    let idempotency_key = uuid::Uuid::new_v4();

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
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
        )))
}
