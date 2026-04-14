use anyhow::{Context, Result};
use reqwest::Client;
use std::time::Duration;

fn make_client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/120.0")
        .build()
        .context("HTTP 클라이언트 생성 실패")
}

/// URL 콘텐츠 가져오기
pub async fn web_fetch(url: &str) -> Result<String> {
    let client = make_client()?;
    let resp = client.get(url).send().await
        .with_context(|| format!("URL 요청 실패: {}", url))?;

    if !resp.status().is_success() {
        anyhow::bail!("HTTP {}: {}", resp.status(), url);
    }

    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let body = resp.text().await.context("응답 읽기 실패")?;

    let is_html = content_type.contains("html")
        || body.trim_start().starts_with("<!DOCTYPE")
        || body.trim_start().starts_with("<html");

    let result = if is_html {
        strip_html(&body)
    } else {
        body
    };

    if result.len() > 8000 {
        Ok(format!("{}\n\n[... 내용이 잘렸습니다 (총 {}자)]",
            crate::utils::trunc(&result, 8000), result.len()))
    } else {
        Ok(result)
    }
}

/// DuckDuckGo 웹 검색 (Instant API + HTML 폴백)
pub async fn web_search(query: &str) -> Result<String> {
    let client = make_client()?;

    // 1차: DuckDuckGo Instant Answer API
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
        .context("검색 요청 실패")?;

    let body: serde_json::Value = resp.json().await.context("검색 결과 파싱 실패")?;

    let mut output = Vec::new();

    // 즉답
    if let Some(answer) = body["Answer"].as_str() {
        if !answer.is_empty() {
            output.push(format!("답변: {}", answer));
        }
    }

    // 요약
    if let Some(text) = body["Abstract"].as_str() {
        if !text.is_empty() {
            output.push(format!("요약: {}", text));
            if let Some(src) = body["AbstractSource"].as_str() {
                if !src.is_empty() {
                    output.push(format!("출처: {} ({})",
                        src, body["AbstractURL"].as_str().unwrap_or("")));
                }
            }
        }
    }

    // 관련 항목
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

    // Instant API 결과가 없으면 HTML 검색 시도
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
            "'{}' 검색 결과 없음.\n검색 URL: https://duckduckgo.com/?q={}",
            query,
            query.replace(' ', "+")
        ))
    } else {
        Ok(output.join("\n"))
    }
}

/// DuckDuckGo HTML 결과 Parsing (폴백)
async fn search_html_fallback(client: &Client, query: &str) -> Result<Vec<String>> {
    let url = format!(
        "https://html.duckduckgo.com/html/?q={}",
        query.replace(' ', "+")
    );
    let resp = client.get(&url).send().await?;
    let html = resp.text().await?;

    let mut results = vec![format!("검색: '{}'", query)];
    let text = strip_html_search(&html);

    // 텍스트에서 결과 추출 (간단히 첫 3000자)
    let preview: String = text.chars().take(3000).collect();
    if !preview.trim().is_empty() {
        results.push(preview);
    }

    Ok(results)
}

fn strip_html_search(html: &str) -> String {
    // script/style 제거
    let s = remove_tag_block(html, "script");
    let s = remove_tag_block(&s, "style");

    // 태그 제거
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

    // 공백/줄바꿈 정리
    text.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && l.len() > 10)
        .collect::<Vec<_>>()
        .join("\n")
}

/// HTML 태그 제거, 텍스트만 추출
fn strip_html(html: &str) -> String {
    // script / style 블록 제거
    let s = remove_tag_block(html, "script");
    let s = remove_tag_block(&s, "style");
    let s = remove_tag_block(&s, "noscript");

    // 태그 제거
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

    // HTML 엔티티 변환
    let text = text
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .replace("&mdash;", "—")
        .replace("&ndash;", "–");

    // 빈 줄 정리
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
