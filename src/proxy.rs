use axum::extract::Path;

pub async fn proxy(Path(_url): Path<String>) -> &'static str {
    "todo!"
}
