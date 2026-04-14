use anyhow::{Context, Result};
use reqwest::Client;
use std::time::Duration;

fn make_client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/120.0")
        .build()
        .context("Failed to create HTTP client")
}

/// Fetch content from a URL
pub async fn web_fetch(url: &str) -> Result<String> {
    let client = make_client()?;
    let resp = client.get(url).send().await
        .with_context(|| format!("Request failed: {}", url))?;

    if !resp.status().is_success() {
        anyhow::bail!("HTTP {}: {}", resp.status(), url);
    }

    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let body = resp.text().await.context("Failed to read response")?;

    let is_html = content_type.contains("html")
        || body.trim_start().starts_with("<!DOCTYPE")
        || body.trim_start().starts_with("<html");

    let result = if is_html {
        strip_html(&body)
    } else {
        body
    };

    if result.len() > 8000 {
        Ok(format!("{}\n\n[... content truncated (total {} chars)]",
            crate::utils::trunc(&result, 8000), result.len()))
    } else {
        Ok(result)
    }
}

/// DuckDuckGo web search (Instant API + HTML fallback)
pub async fn web_search(query: &str) -> Result<String> {
    let client = make_client()?;

    // First: DuckDuckGo Instant Answer API
    let resp = client
        .get("https://api.duckduckgo.com/")
        .query(&[
            ("q", query),
            ("format", "json"),
            ("no_html", "1"),
            ("skip_disambig", "1"),
        ])
        .send()
        .await
        .context("Search request failed")?;

    let body: serde_json::Value = resp.json().await.context("Failed to parse search results")?;

    let mut output = Vec::new();

    // Direct answer
    if let Some(answer) = body["Answer"].as_str() {
        if !answer.is_empty() {
            output.push(format!("Answer: {}", answer));
        }
    }

    // Summary
    if let Some(text) = body["Abstract"].as_str() {
        if !text.is_empty() {
            output.push(format!("Summary: {}", text));
            if let Some(src) = body["AbstractSource"].as_str() {
                if !src.is_empty() {
                    output.push(format!("Source: {} ({})",
                        src, body["AbstractURL"].as_str().unwrap_or("")));
                }
            }
        }
    }

    // Related topics
    if let Some(topics) = body["RelatedTopics"].as_array() {
        let mut count = 0;
        for topic in topics {
            if count >= 5 { break; }
            if let Some(text) = topic["Text"].as_str() {
                if !text.is_empty() {
                    let url = topic["FirstURL"].as_str().unwrap_or("");
                    if url.is_empty() {
                        output.push(format!("• {}", text));
                    } else {
                        output.push(format!("• {} → {}", text, url));
                    }
                    count += 1;
                }
            }
        }
    }

    // Fall back to HTML search if Instant API returns nothing
    if output.is_empty() {
        match search_html_fallback(&client, query).await {
            Ok(results) if !results.is_empty() => {
                output.extend(results);
            }
            _ => {}
        }
    }

    if output.is_empty() {
        Ok(format!(
            "No results found for '{}'.\nSearch URL: https://duckduckgo.com/?q={}",
            query,
            query.replace(' ', "+")
        ))
    } else {
        Ok(output.join("\n"))
    }
}

/// DuckDuckGo HTML result parsing (fallback)
async fn search_html_fallback(client: &Client, query: &str) -> Result<Vec<String>> {
    let url = format!(
        "https://html.duckduckgo.com/html/?q={}",
        query.replace(' ', "+")
    );
    let resp = client.get(&url).send().await?;
    let html = resp.text().await?;

    let mut results = vec![format!("Search: '{}'", query)];
    let text = strip_html_search(&html);

    // extract results from text (first 3000 chars)
    let preview: String = text.chars().take(3000).collect();
    if !preview.trim().is_empty() {
        results.push(preview);
    }

    Ok(results)
}

fn strip_html_search(html: &str) -> String {
    // remove script/style blocks
    let s = remove_tag_block(html, "script");
    let s = remove_tag_block(&s, "style");

    // remove tags
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

    // normalize whitespace/newlines
    text.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && l.len() > 10)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Strip HTML tags and extract plain text
fn strip_html(html: &str) -> String {
    // remove script/style blocks
    let s = remove_tag_block(html, "script");
    let s = remove_tag_block(&s, "style");
    let s = remove_tag_block(&s, "noscript");

    // remove tags
    let mut text = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => text.push(ch),
            _ => {}
        }
    }

    // decode HTML entities
    let text = text
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .replace("&mdash;", "—")
        .replace("&ndash;", "–");

    // clean up empty lines
    text.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn remove_tag_block(html: &str, tag: &str) -> String {
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    let mut result = String::new();
    let mut rest = html;

    loop {
        let lower_rest = rest.to_lowercase();
        match lower_rest.find(&open) {
            Some(start) => {
                result.push_str(&rest[..start]);
                match lower_rest[start..].find(&close) {
                    Some(end) => {
                        rest = &rest[start + end + close.len()..];
                    }
                    None => {
                        break;
                    }
                }
            }
            None => {
                result.push_str(rest);
                break;
            }
        }
    }

    result
}
