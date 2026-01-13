use axum::response::Html;
use std::fmt::Write;

use crate::session_state::{FlashLevel, TypedSession};

pub async fn login_form(session: TypedSession) -> Html<String> {
    let mut error_html = String::new();

    for m in session
        .get_flash_messages()
        .await
        .into_iter()
    {
        writeln!(error_html, "<p><i>{}</i></p>", m.content).unwrap();
    }

    Html(format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8">
    <title>Login</title>
</head>
<body>
    {error_html}
    <form action="/login" method="post">
        <label>Username
            <input
                type="text"
                placeholder="Enter Username"
                name="username"
            >
        </label>
        <label>Password
            <input
                type="password"
                placeholder="Enter Password"
                name="password"
            >
        </label>
        <button type="submit">Login</button>
    </form>
</body>
</html>"#,
    ))
}
