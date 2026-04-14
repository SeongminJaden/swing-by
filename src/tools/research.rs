/// 리서치 툴: 웹 검색 + 문서 패치 + 패키지 정보 통합
///
/// 프로젝트 생성 시 최신 라이브러리/문서를 동시에 검색하고 분석

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

// ─── 심층 검색: 검색 + 상위 N개 URL 동시 패치 ────────────────────────────────

/// 검색어로 검색 후 상위 결과들의 내용을 동시에 가져와 통합
pub async fn research(query: &str, max_pages: usize) -> Result<String> {
    let max_pages = max_pages.min(5).max(1);
    let client = make_client();

    // 1단계: DuckDuckGo에서 URL 목록 수집
    let urls = collect_search_urls(&client, query, max_pages).await;

    if urls.is_empty() {
        // Instant API 폴백
        let instant = duckduckgo_instant(&client, query).await
            .unwrap_or_else(|_| format!("검색 결과 없음: {}", query));
        return Ok(instant);
    }

    // 2단계: 모든 URL 동시 패치 (tokio::join)
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
        format!("=== 검색: '{}' ===\n", query),
    ];

    for (url, handle) in fetch_handles {
        match handle.await {
            Ok(Ok(content)) if !content.trim().is_empty() => {
                let snippet: String = content.chars().take(MAX_PER_PAGE).collect();
                results.push(format!("--- 출처: {} ---\n{}", url, snippet));
            }
            Ok(Err(e)) => {
                results.push(format!("--- {} (접근 실패: {}) ---", url, e));
            }
            _ => {}
        }
    }

    let combined = results.join("\n\n");
    if combined.len() > MAX_TOTAL {
        Ok(format!("{}\n\n[총 {}자 중 일부]",
            crate::utils::trunc(&combined, MAX_TOTAL), combined.len()))
    } else {
        Ok(combined)
    }
}

/// DuckDuckGo HTML 페이지에서 URL 목록 추출
async fn collect_search_urls(client: &Client, query: &str, n: usize) -> Vec<String> {
    // 방법 1: DuckDuckGo Instant API에서 RelatedTopics URL 추출
    let mut urls = vec![];

    if let Ok(resp) = client
        .get("https://api.duckduckgo.com/")
        .query(&[("q", query), ("format", "json"), ("no_html", "1")])
        .send().await
    {
        if let Ok(body) = resp.json::<serde_json::Value>().await {
            // AbstractURL (위키피디아 등)
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

    // 방법 2: DuckDuckGo HTML 페이지 스크래핑 (Instant API 결과 부족 시)
    if urls.len() < n {
        if let Ok(additional) = scrape_ddg_html(client, query, n).await {
            for u in additional {
                if !urls.contains(&u) { urls.push(u); }
            }
        }
    }

    urls.into_iter().take(n).collect()
}

/// DuckDuckGo HTML 결과 페이지에서 링크 추출
async fn scrape_ddg_html(client: &Client, query: &str, n: usize) -> Result<Vec<String>> {
    let url = format!(
        "https://html.duckduckgo.com/html/?q={}",
        urlencoding::encode(query)
    );
    let resp = client.get(&url).send().await?;
    let html = resp.text().await?;

    let mut urls = vec![];
    // href 추출 (간단한 파서)
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

/// DuckDuckGo Instant API 결과
async fn duckduckgo_instant(client: &Client, query: &str) -> Result<String> {
    let resp = client
        .get("https://api.duckduckgo.com/")
        .query(&[("q", query), ("format", "json"), ("no_html", "1"), ("skip_disambig", "1")])
        .send().await?;
    let body: serde_json::Value = resp.json().await?;

    let mut out = vec![];
    if let Some(a) = body["Answer"].as_str() { if !a.is_empty() { out.push(format!("답변: {}", a)); } }
    if let Some(a) = body["Abstract"].as_str() { if !a.is_empty() { out.push(format!("요약: {}", a)); } }
    if let Some(src) = body["AbstractSource"].as_str() {
        if !src.is_empty() {
            let url = body["AbstractURL"].as_str().unwrap_or("");
            out.push(format!("출처: {} — {}", src, url));
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

/// URL 패치 후 HTML 정리
async fn fetch_and_clean(client: &Client, url: &str) -> Result<String> {
    let resp = client.get(url).send().await
        .with_context(|| format!("패치 실패: {}", url))?;

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

// ─── 패키지 레지스트리 정보 ───────────────────────────────────────────────────

/// 패키지 최신 버전 및 메타데이터 조회
pub async fn pkg_info(ecosystem: &str, package: &str) -> Result<String> {
    let client = make_client();
    match ecosystem.to_lowercase().as_str() {
        "npm" | "node" | "js" => npm_info(&client, package).await,
        "pip" | "pypi" | "python" | "py" => pypi_info(&client, package).await,
        "cargo" | "rust" | "crates" => crates_info(&client, package).await,
        "go" | "golang" => go_pkg_info(&client, package).await,
        "gem" | "ruby" => rubygems_info(&client, package).await,
        other => anyhow::bail!(
            "지원하지 않는 생태계: '{}'. 지원: npm, pip/pypi, cargo/crates, go, gem/ruby", other
        ),
    }
}

async fn npm_info(client: &Client, package: &str) -> Result<String> {
    let url = format!("https://registry.npmjs.org/{}/latest", package);
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("npm 패키지 '{}' 없음", package);
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
        format!("최신 버전: {}", version),
        format!("라이선스: {}", license),
    ];
    if !desc.is_empty() { out.push(format!("설명: {}", desc)); }
    if !homepage.is_empty() { out.push(format!("홈페이지: {}", homepage)); }
    if !peers.is_empty() { out.push(format!("peerDeps: {}", peers.join(", "))); }

    // weekly downloads
    let dl_url = format!("https://api.npmjs.org/downloads/point/last-week/{}", package);
    if let Ok(dl_resp) = client.get(&dl_url).send().await {
        if let Ok(dl_data) = dl_resp.json::<serde_json::Value>().await {
            if let Some(dl) = dl_data["downloads"].as_u64() {
                out.push(format!("주간 다운로드: {}", dl));
            }
        }
    }

    Ok(out.join("\n"))
}

async fn pypi_info(client: &Client, package: &str) -> Result<String> {
    let url = format!("https://pypi.org/pypi/{}/json", package);
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("PyPI 패키지 '{}' 없음", package);
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
        format!("최신 버전: {}", version),
        format!("Python 요구: {}", requires_python),
        format!("라이선스: {}", license),
    ];
    if !desc.is_empty() { out.push(format!("설명: {}", desc)); }
    if !homepage.is_empty() { out.push(format!("홈페이지: {}", homepage)); }

    Ok(out.join("\n"))
}

async fn crates_info(client: &Client, package: &str) -> Result<String> {
    let url = format!("https://crates.io/api/v1/crates/{}", package);
    let resp = client.get(&url)
        .header("User-Agent", "ai-agent/1.0")
        .send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("crates.io 패키지 '{}' 없음", package);
    }
    let data: serde_json::Value = resp.json().await?;
    let krate = &data["crate"];
    let version = krate["newest_version"].as_str().unwrap_or("?");
    let desc = krate["description"].as_str().unwrap_or("");
    let downloads = krate["downloads"].as_u64().unwrap_or(0);
    let homepage = krate["homepage"].as_str().unwrap_or("");
    let repo = krate["repository"].as_str().unwrap_or("");
    let license = data["versions"][0]["license"].as_str().unwrap_or("?");

    // 최신 버전 features 조회
    let mut out = vec![
        format!("🦀 crates.io/{}", package),
        format!("최신 버전: {}", version),
        format!("총 다운로드: {}", downloads),
        format!("라이선스: {}", license),
    ];
    if !desc.is_empty() { out.push(format!("설명: {}", desc)); }
    if !homepage.is_empty() { out.push(format!("홈페이지: {}", homepage)); }
    if !repo.is_empty() { out.push(format!("레포: {}", repo)); }

    // Cargo.toml 추가 방법
    out.push(format!("\n# Cargo.toml에 추가:\n{} = \"{}\"", package, version));

    Ok(out.join("\n"))
}

async fn go_pkg_info(client: &Client, package: &str) -> Result<String> {
    // pkg.go.dev API
    let url = format!("https://pkg.go.dev/{}?tab=overview", package);
    let text = fetch_and_clean(client, &url).await
        .unwrap_or_else(|_| format!("Go 패키지: {}", package));

    let snippet: String = text.chars().take(3000).collect();
    Ok(format!("🐹 Go/{}\n출처: https://pkg.go.dev/{}\n\n{}", package, package, snippet))
}

async fn rubygems_info(client: &Client, package: &str) -> Result<String> {
    let url = format!("https://rubygems.org/api/v1/gems/{}.json", package);
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("RubyGems '{}' 없음", package);
    }
    let data: serde_json::Value = resp.json().await?;
    let version = data["version"].as_str().unwrap_or("?");
    let desc = data["info"].as_str().unwrap_or("");
    let homepage = data["homepage_uri"].as_str().unwrap_or("");
    let downloads = data["downloads"].as_u64().unwrap_or(0);

    let mut out = vec![
        format!("💎 RubyGems/{}", package),
        format!("최신 버전: {}", version),
        format!("총 다운로드: {}", downloads),
    ];
    if !desc.is_empty() { out.push(format!("설명: {}", &desc.chars().take(200).collect::<String>())); }
    if !homepage.is_empty() { out.push(format!("홈페이지: {}", homepage)); }
    out.push(format!("\n# Gemfile에 추가:\ngem '{}', '~> {}'", package, version));

    Ok(out.join("\n"))
}

// ─── 최신 버전 일괄 조회 ─────────────────────────────────────────────────────

/// 여러 패키지의 최신 버전을 동시에 조회
pub async fn pkg_versions_bulk(ecosystem: &str, packages: &[&str]) -> Result<String> {
    let mut handles = vec![];
    for &pkg in packages {
        let eco = ecosystem.to_string();
        let p = pkg.to_string();
        let handle = tokio::spawn(async move {
            pkg_info(&eco, &p).await
                .unwrap_or_else(|e| format!("  {}: 조회 실패 ({})", p, e))
        });
        handles.push(handle);
    }

    let mut results = vec![format!("=== {} 패키지 최신 버전 ===", ecosystem)];
    for handle in handles {
        if let Ok(info) = handle.await {
            results.push(info);
        }
    }
    Ok(results.join("\n\n"))
}

// ─── 공식 문서 패치 ──────────────────────────────────────────────────────────

/// 공식 문서 URL을 지정하여 내용을 깔끔하게 가져옴
pub async fn docs_fetch(url: &str, max_chars: usize) -> Result<String> {
    let client = make_client();
    let max_chars = max_chars.max(1000).min(15000);

    let content = fetch_and_clean(&client, url).await?;
    let snippet: String = content.chars().take(max_chars).collect();

    Ok(format!("=== 문서: {} ===\n{}", url, snippet))
}

// ─── HTML 파서 ────────────────────────────────────────────────────────────────

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

// ─── URL 인코딩 Helpers ─────────────────────────────────────────────────────────

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
