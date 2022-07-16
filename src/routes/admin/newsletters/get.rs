use actix_web::{http::header::ContentType, HttpResponse};

pub async fn newsletters_form() -> Result<HttpResponse, actix_web::Error> {

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
            <button type="submit">Submit</button>
        </form>
            </body>
            </html>"#,
        )))
}
