//! HTML login page for OAuth authorization.

/// Render the authorization login page.
///
/// All parameters are HTML-escaped to prevent XSS.
pub fn render_login_page(
    client_name: &str,
    client_id: &str,
    redirect_uri: &str,
    state: &str,
    code_challenge: &str,
    scope: &str,
    error_message: Option<&str>,
) -> String {
    let error_html = error_message
        .map(|msg| {
            format!(
                r#"<div style="background:#fee;border:1px solid #c00;color:#c00;padding:10px;border-radius:4px;margin-bottom:16px">{}</div>"#,
                html_escape(msg)
            )
        })
        .unwrap_or_default();

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Authorize - Semantic Scholar MCP</title>
<style>
body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; background: #f5f5f5; margin: 0; display: flex; justify-content: center; align-items: center; min-height: 100vh; }}
.card {{ background: #fff; border-radius: 8px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); padding: 32px; max-width: 400px; width: 100%; }}
h1 {{ font-size: 20px; margin: 0 0 8px; color: #333; }}
.subtitle {{ color: #666; font-size: 14px; margin: 0 0 24px; }}
label {{ display: block; font-size: 14px; font-weight: 500; margin-bottom: 6px; color: #333; }}
input[type="password"] {{ width: 100%; padding: 10px; border: 1px solid #ddd; border-radius: 4px; font-size: 14px; box-sizing: border-box; }}
input[type="password"]:focus {{ outline: none; border-color: #4a90d9; box-shadow: 0 0 0 2px rgba(74,144,217,0.2); }}
button {{ width: 100%; padding: 10px; background: #4a90d9; color: #fff; border: none; border-radius: 4px; font-size: 14px; font-weight: 500; cursor: pointer; margin-top: 16px; }}
button:hover {{ background: #357abd; }}
</style>
</head>
<body>
<div class="card">
<h1>Semantic Scholar MCP</h1>
<p class="subtitle"><strong>{client_name}</strong> is requesting access</p>
{error_html}
<form method="POST" action="/authorize">
<input type="hidden" name="client_id" value="{client_id_escaped}">
<input type="hidden" name="redirect_uri" value="{redirect_uri_escaped}">
<input type="hidden" name="state" value="{state_escaped}">
<input type="hidden" name="code_challenge" value="{code_challenge_escaped}">
<input type="hidden" name="scope" value="{scope_escaped}">
<label for="password">Server Password</label>
<input type="password" id="password" name="password" placeholder="Enter MCP server password" required autofocus>
<button type="submit">Approve</button>
</form>
</div>
</body>
</html>"#,
        client_name = html_escape(client_name),
        error_html = error_html,
        client_id_escaped = html_escape(client_id),
        redirect_uri_escaped = html_escape(redirect_uri),
        state_escaped = html_escape(state),
        code_challenge_escaped = html_escape(code_challenge),
        scope_escaped = html_escape(scope),
    )
}

/// Escape HTML special characters.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_escape() {
        assert_eq!(
            html_escape(r#"<script>alert("xss")</script>"#),
            "&lt;script&gt;alert(&quot;xss&quot;)&lt;/script&gt;"
        );
    }

    #[test]
    fn test_render_without_error() {
        let html = render_login_page(
            "Test App",
            "client123",
            "http://localhost/cb",
            "state1",
            "challenge1",
            "mcp",
            None,
        );
        assert!(html.contains("Test App"));
        assert!(html.contains("client123"));
        assert!(!html.contains("background:#fee"));
    }

    #[test]
    fn test_render_with_error() {
        let html = render_login_page("App", "id", "uri", "st", "ch", "sc", Some("Wrong password"));
        assert!(html.contains("Wrong password"));
        assert!(html.contains("background:#fee"));
    }
}
