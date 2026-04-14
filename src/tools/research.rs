/// Research tool: web search + document fetch + package info combined
///
/// Searches and analyzes the latest libraries/docs in parallel when scaffolding a project

use anyhow::{Context, Result};
use reqwest::Client;
use std::time::Duration;

const FETCH_TIMEOUT: u64 = 20;
const MAX_PER_PAGE: usize = 6000;
const MAX_TOTAL: usize = 20000;

fn make_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(FETCH_TIMEOUT))
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36")
        .build()
        .unwrap_or_default()
}

// ─── Deep search: query + concurrent fetch of top N URLs ──────────────────────────────────

/// Search for a query and concurrently fetch + merge the top results
pub async fn research(query: &str, max_pages: usize) -> Result<String> {
    let max_pages = max_pages.min(5).max(1);
    let client = make_client();

    // Step 1: collect URL list from DuckDuckGo
    let urls = collect_search_urls(&client, query, max_pages).await;

    if urls.is_empty() {
        // Instant API fallback
        let instant = duckduckgo_instant(&client, query).await
            .unwrap_or_else(|_| format!("No results found: {}", query));
        return Ok(instant);
    }

    // Step 2: concurrently fetch all URLs
    let mut fetch_handles = vec![];
    for url in urls.iter().take(max_pages) {
        let client = client.clone();
        let url_key = url.clone();
        let handle = tokio::spawn(async move {
            fetch_and_clean(&client, &url_key).await
        });
        fetch_handles.push((url.clone(), handle));
    }

    let mut results = vec![
        format!("=== Search: '{}' ===\n", query),
    ];

    for (url, handle) in fetch_handles {
        match handle.await {
            Ok(Ok(content)) if !content.trim().is_empty() => {
                let snippet: String = content.chars().take(MAX_PER_PAGE).collect();
                results.push(format!("--- Source: {} ---\n{}", url, snippet));
            }
            Ok(Err(e)) => {
                results.push(format!("--- {} (fetch failed: {}) ---", url, e));
            }
            _ => {}
        }
    }

    let combined = results.join("\n\n");
    if combined.len() > MAX_TOTAL {
Ok(format!("{}\n\n[partial content, {} chars total]",
            crate::utils::trunc(&combined, MAX_TOTAL), combined.len()))
    } else {
        Ok(combined)
    }
}

/// Collect search result URLs from DuckDuckGo
async fn collect_search_urls(client: &Client, query: &str, n: usize) -> Vec<String> {
    // Method 1: extract RelatedTopics URLs from DuckDuckGo Instant API
    let mut urls = vec![];

    if let Ok(resp) = client
        .get("https://api.duckduckgo.com/")
        .query(&[("q", query), ("format", "json"), ("no_html", "1")])
        .send().await
    {
        if let Ok(body) = resp.json::<serde_json::Value>().await {
            // AbstractURL (Wikipedia etc.)
            if let Some(au) = body["AbstractURL"].as_str() {
                if !au.is_empty() { urls.push(au.to_string()); }
            }
            // RelatedTopics FirstURL
            if let Some(topics) = body["RelatedTopics"].as_array() {
                for t in topics.iter().take(n + 2) {
                    if let Some(u) = t["FirstURL"].as_str() {
                        if !u.is_empty() && !urls.contains(&u.to_string()) {
                            urls.push(u.to_string());
                        }
                    }
                }
            }
            // Results
            if let Some(results) = body["Results"].as_array() {
                for r in results.iter().take(n) {
                    if let Some(u) = r["FirstURL"].as_str() {
                        if !u.is_empty() { urls.push(u.to_string()); }
                    }
                }
            }
        }
    }

    // Method 2: scrape DuckDuckGo HTML page (fallback when Instant API yields too few results)
    if urls.len() < n {
        if let Ok(additional) = scrape_ddg_html(client, query, n).await {
            for u in additional {
                if !urls.contains(&u) { urls.push(u); }
            }
        }
    }

    urls.into_iter().take(n).collect()
}

/// Extract links from DuckDuckGo HTML result page
async fn scrape_ddg_html(client: &Client, query: &str, n: usize) -> Result<Vec<String>> {
    let url = format!(
        "https://html.duckduckgo.com/html/?q={}",
        urlencoding::encode(query)
    );
    let resp = client.get(&url).send().await?;
    let html = resp.text().await?;

    let mut urls = vec![];
    // extract hrefs (simple parser)
    let mut rest = html.as_str();
    while let Some(pos) = rest.find("uddg=") {
        rest = &rest[pos + 5..];
        let end = rest.find('"').unwrap_or(rest.len().min(300));
        let encoded = &rest[..end];
        if let Ok(decoded) = urlencoding::decode(encoded) {
            let u = decoded.to_string();
            if u.starts_with("http") && !u.contains("duckduckgo.com") {
                urls.push(u);
                if urls.len() >= n { break; }
            }
        }
    }
    Ok(urls)
}

/// DuckDuckGo Instant API result
async fn duckduckgo_instant(client: &Client, query: &str) -> Result<String> {
    let resp = client
        .get("https://api.duckduckgo.com/")
        .query(&[("q", query), ("format", "json"), ("no_html", "1"), ("skip_disambig", "1")])
        .send().await?;
    let body: serde_json::Value = resp.json().await?;

    let mut out = vec![];
    if let Some(a) = body["Answer"].as_str() { if !a.is_empty() { out.push(format!("Answer: {}", a)); } }
    if let Some(a) = body["Abstract"].as_str() { if !a.is_empty() { out.push(format!("Summary: {}", a)); } }
    if let Some(src) = body["AbstractSource"].as_str() {
        if !src.is_empty() {
            let url = body["AbstractURL"].as_str().unwrap_or("");
            out.push(format!("Source: {} — {}", src, url));
        }
    }
    if let Some(topics) = body["RelatedTopics"].as_array() {
        for t in topics.iter().take(5) {
            if let Some(text) = t["Text"].as_str() {
                if !text.is_empty() {
                    out.push(format!("• {}", text));
                }
            }
        }
    }
    Ok(out.join("\n"))
}

/// Fetch a URL and clean up the HTML
async fn fetch_and_clean(client: &Client, url: &str) -> Result<String> {
    let resp = client.get(url).send().await
        .with_context(|| format!("Fetch failed: {}", url))?;

    if !resp.status().is_success() {
        anyhow::bail!("HTTP {}", resp.status());
    }

    let ct = resp.headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let body = resp.text().await?;
    let is_html = ct.contains("html")
        || body.trim_start().starts_with('<');

    let text = if is_html { strip_html_clean(&body) } else { body };
    Ok(text)
}

// ─── Package registry information ─────────────────────────────────────────────────────────

/// Fetch the latest version and metadata for a package
pub async fn pkg_info(ecosystem: &str, package: &str) -> Result<String> {
    let client = make_client();
    match ecosystem.to_lowercase().as_str() {
        "npm" | "node" | "js" => npm_info(&client, package).await,
        "pip" | "pypi" | "python" | "py" => pypi_info(&client, package).await,
        "cargo" | "rust" | "crates" => crates_info(&client, package).await,
        "go" | "golang" => go_pkg_info(&client, package).await,
        "gem" | "ruby" => rubygems_info(&client, package).await,
        other => anyhow::bail!(
            "Unsupported ecosystem: '{}'. Supported: npm, pip/pypi, cargo/crates, go, gem/ruby", other
        ),
    }
}

async fn npm_info(client: &Client, package: &str) -> Result<String> {
    let url = format!("https://registry.npmjs.org/{}/latest", package);
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("npm package '{}' not found", package);
    }
    let data: serde_json::Value = resp.json().await?;
    let version = data["version"].as_str().unwrap_or("?");
    let desc = data["description"].as_str().unwrap_or("");
    let homepage = data["homepage"].as_str().unwrap_or("");
    let license = data["license"].as_str().unwrap_or("?");

    // peer deps
    let peers: Vec<String> = data["peerDependencies"]
        .as_object().into_iter().flat_map(|m| {
            m.iter().map(|(k, v)| format!("{}: {}", k, v.as_str().unwrap_or("*")))
        }).collect();

    let mut out = vec![
        format!("📦 npm/{}", package),
        format!("Latest version: {}", version),
        format!("License: {}", license),
    ];
    if !desc.is_empty() { out.push(format!("Description: {}", desc)); }
    if !homepage.is_empty() { out.push(format!("Homepage: {}", homepage)); }
    if !peers.is_empty() { out.push(format!("peerDeps: {}", peers.join(", "))); }

    // weekly downloads
    let dl_url = format!("https://api.npmjs.org/downloads/point/last-week/{}", package);
    if let Ok(dl_resp) = client.get(&dl_url).send().await {
        if let Ok(dl_data) = dl_resp.json::<serde_json::Value>().await {
            if let Some(dl) = dl_data["downloads"].as_u64() {
                out.push(format!("Weekly downloads: {}", dl));
            }
        }
    }

    Ok(out.join("\n"))
}

async fn pypi_info(client: &Client, package: &str) -> Result<String> {
    let url = format!("https://pypi.org/pypi/{}/json", package);
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("PyPI package '{}' not found", package);
    }
    let data: serde_json::Value = resp.json().await?;
    let info = &data["info"];
    let version = info["version"].as_str().unwrap_or("?");
    let desc = info["summary"].as_str().unwrap_or("");
    let homepage = info["home_page"].as_str()
        .or_else(|| info["project_url"].as_str())
        .unwrap_or("");
    let license = info["license"].as_str().unwrap_or("?");
    let requires_python = info["requires_python"].as_str().unwrap_or("?");

    let mut out = vec![
        format!("🐍 PyPI/{}", package),
        format!("Latest version: {}", version),
        format!("Python requires: {}", requires_python),
        format!("License: {}", license),
    ];
    if !desc.is_empty() { out.push(format!("Description: {}", desc)); }
    if !homepage.is_empty() { out.push(format!("Homepage: {}", homepage)); }

    Ok(out.join("\n"))
}

async fn crates_info(client: &Client, package: &str) -> Result<String> {
    let url = format!("https://crates.io/api/v1/crates/{}", package);
    let resp = client.get(&url)
        .header("User-Agent", "ai-agent/1.0")
        .send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("crates.io package '{}' not found", package);
    }
    let data: serde_json::Value = resp.json().await?;
    let krate = &data["crate"];
    let version = krate["newest_version"].as_str().unwrap_or("?");
    let desc = krate["description"].as_str().unwrap_or("");
    let downloads = krate["downloads"].as_u64().unwrap_or(0);
    let homepage = krate["homepage"].as_str().unwrap_or("");
    let repo = krate["repository"].as_str().unwrap_or("");
    let license = data["versions"][0]["license"].as_str().unwrap_or("?");

    // fetch features for the latest version
    let mut out = vec![
        format!("🦀 crates.io/{}", package),
        format!("Latest version: {}", version),
        format!("Total downloads: {}", downloads),
        format!("License: {}", license),
    ];
    if !desc.is_empty() { out.push(format!("Description: {}", desc)); }
    if !homepage.is_empty() { out.push(format!("Homepage: {}", homepage)); }
    if !repo.is_empty() { out.push(format!("Repo: {}", repo)); }

    // How to add to Cargo.toml
    out.push(format!("\n# Add to Cargo.toml:\n{} = \"{}\"", package, version));

    Ok(out.join("\n"))
}

async fn go_pkg_info(client: &Client, package: &str) -> Result<String> {
    // pkg.go.dev API
    let url = format!("https://pkg.go.dev/{}?tab=overview", package);
    let text = fetch_and_clean(client, &url).await
        .unwrap_or_else(|_| format!("Go package: {}", package));

    let snippet: String = text.chars().take(3000).collect();
    Ok(format!("🐹 Go/{}\nSource: https://pkg.go.dev/{}\n\n{}", package, package, snippet))
}

async fn rubygems_info(client: &Client, package: &str) -> Result<String> {
    let url = format!("https://rubygems.org/api/v1/gems/{}.json", package);
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("RubyGems '{}' not found", package);
    }
    let data: serde_json::Value = resp.json().await?;
    let version = data["version"].as_str().unwrap_or("?");
    let desc = data["info"].as_str().unwrap_or("");
    let homepage = data["homepage_uri"].as_str().unwrap_or("");
    let downloads = data["downloads"].as_u64().unwrap_or(0);

    let mut out = vec![
        format!("💎 RubyGems/{}", package),
        format!("Latest version: {}", version),
        format!("Total downloads: {}", downloads),
    ];
    if !desc.is_empty() { out.push(format!("Description: {}", &desc.chars().take(200).collect::<String>())); }
    if !homepage.is_empty() { out.push(format!("Homepage: {}", homepage)); }
    out.push(format!("\n# Add to Gemfile:\ngem '{}', '~> {}'", package, version));

    Ok(out.join("\n"))
}

// ─── Bulk latest-version lookup ────────────────────────────────────────────────────────────

/// Concurrently fetch the latest version of multiple packages
pub async fn pkg_versions_bulk(ecosystem: &str, packages: &[&str]) -> Result<String> {
    let mut handles = vec![];
    for &pkg in packages {
        let eco = ecosystem.to_string();
        let p = pkg.to_string();
        let handle = tokio::spawn(async move {
            pkg_info(&eco, &p).await
                .unwrap_or_else(|e| format!("  {}: lookup failed ({})", p, e))
        });
        handles.push(handle);
    }

    let mut results = vec![format!("=== {} package latest versions ===", ecosystem)];
    for handle in handles {
        if let Ok(info) = handle.await {
            results.push(info);
        }
    }
    Ok(results.join("\n\n"))
}

// ─── Official documentation fetch ─────────────────────────────────────────────────────────

/// Fetch and clean official documentation from a given URL
pub async fn docs_fetch(url: &str, max_chars: usize) -> Result<String> {
    let client = make_client();
    let max_chars = max_chars.max(1000).min(15000);

    let content = fetch_and_clean(&client, url).await?;
    let snippet: String = content.chars().take(max_chars).collect();

    Ok(format!("=== Docs: {} ===\n{}", url, snippet))
}

// ─── HTML parser ────────────────────────────────────────────────────────────────

pub fn strip_html_clean(html: &str) -> String {
    let s = remove_tag_block(html, "script");
    let s = remove_tag_block(&s, "style");
    let s = remove_tag_block(&s, "noscript");
    let s = remove_tag_block(&s, "nav");
    let s = remove_tag_block(&s, "footer");
    let s = remove_tag_block(&s, "header");
    let s = remove_tag_block(&s, "aside");

    let mut text = String::new();
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => text.push(ch),
            _ => {}
        }
    }

    let text = text
        .replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">")
        .replace("&quot;", "\"").replace("&#39;", "'").replace("&nbsp;", " ")
        .replace("&mdash;", "—").replace("&ndash;", "–").replace("&hellip;", "...");

    text.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && l.len() > 2)
        .collect::<Vec<_>>()
        .join("\n")
}

fn remove_tag_block(html: &str, tag: &str) -> String {
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    let mut result = String::new();
    let mut rest = html;

    loop {
        let lower = rest.to_lowercase();
        match lower.find(&open) {
            Some(start) => {
                result.push_str(&rest[..start]);
                match lower[start..].find(&close) {
                    Some(end) => { rest = &rest[start + end + close.len()..]; }
                    None => break,
                }
            }
            None => { result.push_str(rest); break; }
        }
    }
    result
}

// ─── URL encoding helpers ───────────────────────────────────────────────────────

mod urlencoding {
    pub fn encode(s: &str) -> String {
        let mut out = String::new();
        for b in s.bytes() {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'
                | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
                b' ' => out.push('+'),
                _ => out.push_str(&format!("%{:02X}", b)),
            }
        }
        out
    }

    pub fn decode(s: &str) -> Result<std::borrow::Cow<'static, str>, ()> {
        let mut out = String::new();
        let mut bytes = s.bytes().peekable();
        while let Some(b) = bytes.next() {
            if b == b'%' {
                let h = bytes.next().ok_or(())?;
                let l = bytes.next().ok_or(())?;
                let hex = format!("{}{}", h as char, l as char);
                let byte = u8::from_str_radix(&hex, 16).map_err(|_| ())?;
                out.push(byte as char);
            } else if b == b'+' {
                out.push(' ');
            } else {
                out.push(b as char);
            }
        }
        Ok(std::borrow::Cow::Owned(out))
    }
}
