use super::*;

pub(super) fn builtin_missing_ui_html(detail: &str) -> String {
    let detail = escape_html(detail);
    format!(
        r#"<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="utf-8"/>
    <meta name="viewport" content="width=device-width, initial-scale=1"/>
    <title>CodexManager Web</title>
    <style>
      body {{ font-family: ui-sans-serif, system-ui, -apple-system, Segoe UI, Roboto, Helvetica, Arial; padding: 40px; line-height: 1.5; color: #111; }}
      .box {{ max-width: 860px; margin: 0 auto; border: 1px solid #e5e7eb; border-radius: 12px; padding: 20px 24px; background: #fafafa; }}
      h1 {{ margin: 0 0 8px; font-size: 20px; }}
      p {{ margin: 10px 0; color: #374151; }}
      code {{ background: #111827; color: #f9fafb; padding: 2px 6px; border-radius: 6px; }}
      a {{ color: #2563eb; }}
    </style>
  </head>
  <body>
    <div class="box">
      <h1>前端资源未就绪</h1>
      <p>当前 <code>codexmanager-web</code> 没有找到可用的前端静态资源。</p>
      <p>详情：<code>{detail}</code></p>
      <p>解决方式：</p>
      <p>1) 使用官方发行物（已内置前端资源）；或</p>
      <p>2) 从源码运行：先执行 <code>pnpm -C apps run build:desktop</code>，再设置 <code>CODEXMANAGER_WEB_ROOT=.../apps/out</code> 启动。</p>
      <p>关闭：访问 <a href="/__quit">/__quit</a>。</p>
    </div>
  </body>
</html>
"#
    )
}

pub(super) async fn serve_missing_ui(State(state): State<Arc<AppState>>) -> Html<String> {
    Html((*state.missing_ui_html).clone())
}

pub(super) async fn serve_embedded_index() -> Response {
    serve_embedded_path("index.html")
}

pub(super) async fn serve_embedded_asset(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> Response {
    serve_embedded_path(&path)
}

fn serve_embedded_path(path: &str) -> Response {
    let raw = path.trim_start_matches('/');
    if raw.contains("..") {
        return (StatusCode::BAD_REQUEST, "bad path").into_response();
    }

    let wanted = if raw.is_empty() { "index.html" } else { raw };
    let Some((served_path, bytes)) = resolve_embedded_asset(wanted) else {
        return (StatusCode::NOT_FOUND, "missing ui").into_response();
    };
    let mime = embedded_ui::guess_mime(served_path);

    let mut out = Response::new(axum::body::Body::from(bytes));
    out.headers_mut().insert(
        "content-type",
        axum::http::HeaderValue::from_str(&mime)
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("application/octet-stream")),
    );
    out
}

fn resolve_embedded_asset(path: &str) -> Option<(&str, &'static [u8])> {
    embedded_ui::read_asset_bytes(path)
        .map(|bytes| (path, bytes))
        .or_else(|| embedded_ui::read_asset_bytes("index.html").map(|bytes| ("index.html", bytes)))
}

#[cfg(all(test, feature = "embedded-ui"))]
mod tests {
    use super::*;

    #[test]
    fn spa_route_fallback_uses_html_content_type() {
        let response = serve_embedded_path("accounts");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("text/html")
        );
    }
}
