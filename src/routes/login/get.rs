use axum::response::Html;
use axum_extra::extract::{CookieJar, cookie::Cookie};

pub async fn login_form(jar: CookieJar) -> (CookieJar, Html<String>) {
    let error_html = match jar.get("_flash") {
        Some(cookie) => format!(r#"<p><i>{}</i></p>"#, cookie.value()),
        None => String::new(),
    };

    let jar = jar.remove(Cookie::from("_flash"));

    (
        jar,
        Html(
            include_str!("login.html")
                .to_string()
                .replace("{{ERROR}}", &error_html),
        ),
    )
}
