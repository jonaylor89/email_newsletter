use askama::Template;

#[derive(Template)]
#[template(path = "emails/confirmation.html")]
pub struct ConfirmationEmailHtml {
    pub subscriber_name: String,
    pub confirmation_link: String,
}

#[derive(Template)]
#[template(path = "emails/confirmation.txt")]
pub struct ConfirmationEmailText {
    pub subscriber_name: String,
    pub confirmation_link: String,
}

#[derive(Template)]
#[template(path = "emails/already_subscribed.html")]
pub struct AlreadySubscribedEmailHtml {
    pub subscriber_name: String,
}

#[derive(Template)]
#[template(path = "emails/already_subscribed.txt")]
pub struct AlreadySubscribedEmailText {
    pub subscriber_name: String,
}
