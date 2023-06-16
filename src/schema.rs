use std::error::Error;

use serde_json::{json, Value};

use crate::articles::Article;

const SUBORDINATE_TYPES: &[&str] = &[
    "Person",
    "WebPage",
    "ProfilePage",
    "WebPageElement",
    "BlogPosting",
    "BreadcrumbList",
    "Blog",
    "ItemList",
    "ListItem",
    "SiteNavigationElement",
];

const PAGE_TYPES: &[&str] = &["WebPage", "ProfilePage", "Blog", "CollectionPage"];
const SUPERIOR_TYPES: &[&str] = &["WebSite"];
const LIST_TYPES: &[&str] = &["BreadcrumbList", "ItemList"];

fn root_id(canonical: &str, fragment: &str) -> String {
    crate::routes::Route::home().id(canonical, fragment)
}

fn tag_display(content: &Value, tag: &str) -> String {
    content["website"]["tags"][tag]
        .as_str()
        .unwrap_or(tag)
        .to_owned()
}

fn site_lang(content: &Value) -> &str {
    content["website"]["lang"].as_str().unwrap_or("")
}

fn label<'a>(content: &'a Value, key: &str) -> &'a str {
    content["website"]["labels"][key].as_str().unwrap_or("")
}

fn page_id(canonical: &str, path: &str, fragment: &str) -> String {
    crate::routes::Route::new(path).id(canonical, fragment)
}

pub fn person_node(content: &Value) -> Value {
    let canonical = content["website"]["canonical"].as_str().unwrap_or("");
    let person = &content["person"];
    let name = person["name"].as_str().unwrap_or("");
    let id = root_id(canonical, crate::routes::SLICE_PERSON);

    json!({
        "@type": "Person",
        "@id": id,
        "url": id,
        "name": name,
        "givenName": name.split_whitespace().next().unwrap_or(""),
        "familyName": name.split_whitespace().nth(1).unwrap_or(""),
        "email": format!("mailto:{}", person["email"].as_str().unwrap_or("")),
        "description": person["description"],
        "sameAs": person["links"]
            .as_array()
            .map(|links| links.iter().filter_map(|l| l["href"].as_str()).collect::<Vec<_>>())
            .unwrap_or_default()
    })
}

pub fn website_node(content: &Value) -> Value {
    let website = &content["website"];
    let canonical = website["canonical"].as_str().unwrap_or("");

    json!({
        "@type": "WebSite",
        "@id": root_id(canonical, "web"),
        "url": canonical,
        "name": website["titles"]["home"],
        "description": website["description"],
        "inLanguage": site_lang(content),
        "publisher": { "@id": root_id(canonical, crate::routes::SLICE_PERSON) }
    })
}

fn blog_url(canonical: &str) -> String {
    crate::routes::Route::blog().id(canonical, crate::routes::SLICE_PAGE)
}

fn blog_node(content: &Value) -> Value {
    let canonical = content["website"]["canonical"].as_str().unwrap_or("");

    json!({
        "@type": "Blog",
        "@id": blog_url(canonical),
        "url": blog_url(canonical),
        "name": label(content, "blog"),
        "isPartOf": { "@id": root_id(canonical, "web") },
        "publisher": { "@id": root_id(canonical, crate::routes::SLICE_PERSON) }
    })
}

pub fn home_graph(content: &Value, recent_articles: &[Article]) -> Value {
    let website = &content["website"];
    let canonical = website["canonical"].as_str().unwrap_or("");

    let mut graph = vec![
        person_node(content),
        website_node(content),
        json!({
            "@type": "ProfilePage",
            "@id": root_id(canonical, "page"),
            "url": root_id(canonical, "page"),
            "name": website["titles"]["home"],
            "description": website["descriptions"]["home"],
            "inLanguage": site_lang(content),
            "isPartOf": { "@id": root_id(canonical, "web") },
            "mainEntity": { "@id": root_id(canonical, crate::routes::SLICE_PERSON) },
            "hasPart": [
                { "@id": root_id(canonical, "blog") },
                { "@id": root_id(canonical, "links") }
            ]
        }),
    ];

    let link_items: Vec<Value> = content["person"]["links"]
        .as_array()
        .map(|links| {
            links
                .iter()
                .enumerate()
                .map(|(i, l)| {
                    let title = l["title"].as_str().unwrap_or("");
                    let anchor = format!("link-{}", title.to_lowercase());
                    json!({
                        "@type": "ListItem",
                        "position": i + 1,
                        "item": {
                            "@type": "WebPage",
                            "@id": root_id(canonical, &anchor),
                            "url": root_id(canonical, &anchor),
                            "name": title
                        }
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    graph.push(json!({
        "@type": "WebPageElement",
        "@id": root_id(canonical, "links"),
        "url": root_id(canonical, "links"),
        "name": label(content, "links"),
        "isPartOf": { "@id": root_id(canonical, "page") },
        "mainEntity": {
            "@type": "ItemList",
            "@id": root_id(canonical, "links-list"),
            "url": root_id(canonical, "links-list"),
            "name": label(content, "links"),
            "numberOfItems": link_items.len(),
            "itemListElement": link_items
        }
    }));

    if !recent_articles.is_empty() {
        let article_items: Vec<Value> = recent_articles
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let article_url = format!("{canonical}{}", p.url());
                json!({
                    "@type": "ListItem",
                    "@id": root_id(canonical, &format!("article-{}", p.slug)),
                    "url": root_id(canonical, &format!("article-{}", p.slug)),
                    "position": i + 1,
                    "item": {
                        "@type": "WebPage",
                        "@id": format!("{article_url}#page"),
                        "url": format!("{article_url}#page"),
                        "name": p.frontmatter.title
                    }
                })
            })
            .collect();

        graph.push(json!({
            "@type": "WebPageElement",
            "@id": root_id(canonical, "blog"),
            "url": root_id(canonical, "blog"),
            "name": label(content, "recent"),
            "isPartOf": { "@id": root_id(canonical, "page") },
            "mainEntity": {
                "@type": "ItemList",
                "@id": root_id(canonical, "blog-articles"),
                "url": root_id(canonical, "blog-articles"),
                "name": label(content, "recent"),
                "numberOfItems": article_items.len(),
                "itemListElement": article_items
            }
        }));
    }

    json!({ "@context": "https://schema.org", "@graph": graph })
}

pub fn article_graph(content: &Value, article: &Article) -> Value {
    let canonical = content["website"]["canonical"].as_str().unwrap_or("");
    let path = article.url();
    let page = |fragment: &str| page_id(canonical, &path, fragment);

    let has_toc = !article.toc.is_empty();
    let toc_items: Vec<Value> = article
        .toc
        .iter()
        .enumerate()
        .map(|(i, item)| {
            json!({
                "@type": "ListItem",
                "position": i + 1,
                "item": {
                    "@type": "WebPageElement",
                    "@id": page(&item.id),
                    "url": page(&item.id),
                    "name": item.text
                }
            })
        })
        .collect();

    let mut web_page = json!({
        "@type": "WebPage",
        "@id": page("page"),
        "url": page("page"),
        "name": article.frontmatter.title,
        "description": article.frontmatter.description,
        "inLanguage": article.lang(),
        "isPartOf": { "@id": root_id(canonical, "web") },
        "breadcrumb": { "@id": page("breadcrumb") },
        "mainEntity": { "@id": page("article") },
        "datePublished": article.date_atom(),
        "dateModified": article.modified_atom
    });
    if has_toc {
        web_page["hasPart"] = json!([{ "@id": page("toc") }]);
    }

    let mut blog_posting = json!({
        "@type": "BlogPosting",
        "@id": page("article"),
        "url": page("article"),
        "headline": article.frontmatter.title,
        "description": article.frontmatter.description,
        "inLanguage": article.lang(),
        "isPartOf": { "@id": page("page") },
        "mainEntityOfPage": { "@id": page("page") },
        "author": { "@id": root_id(canonical, crate::routes::SLICE_PERSON) },
        "publisher": { "@id": root_id(canonical, crate::routes::SLICE_PERSON) },
        "datePublished": article.date_atom(),
        "dateModified": article.modified_atom
    });
    if !article.tags().is_empty() {
        let keywords = article
            .tags()
            .iter()
            .map(|t| tag_display(content, t))
            .collect::<Vec<_>>()
            .join(", ");
        blog_posting["keywords"] = json!(keywords);
    }

    let mut graph = vec![
        person_node(content),
        website_node(content),
        blog_node(content),
        web_page,
        blog_posting,
        json!({
            "@type": "BreadcrumbList",
            "@id": page("breadcrumb"),
            "url": page("breadcrumb"),
            "itemListElement": [
                {
                    "@type": "ListItem",
                    "position": 1,
                    "item": { "@type": "WebPage", "@id": canonical, "url": canonical, "name": label(content, "home") }
                },
                {
                    "@type": "ListItem",
                    "position": 2,
                    "item": {
                        "@type": "WebPage",
                        "@id": blog_url(canonical),
                        "url": blog_url(canonical),
                        "name": label(content, "blog")
                    }
                },
                {
                    "@type": "ListItem",
                    "position": 3,
                    "item": {
                        "@type": "WebPage",
                        "@id": page("article"),
                        "url": page("article"),
                        "name": article.frontmatter.title
                    }
                }
            ]
        }),
    ];

    if has_toc {
        graph.push(json!({
            "@type": ["SiteNavigationElement", "WebPageElement"],
            "@id": page("toc"),
            "url": page("toc"),
            "name": label(content, "toc"),
            "isPartOf": { "@id": page("page") },
            "mainEntity": {
                "@type": "ItemList",
                "@id": page("toc-list"),
                "url": page("toc-list"),
                "name": label(content, "toc_list"),
                "numberOfItems": toc_items.len(),
                "itemListElement": toc_items
            }
        }));
    }

    json!({ "@context": "https://schema.org", "@graph": graph })
}

pub fn blog_index_graph(content: &Value, articles: &[Article], description: &str) -> Value {
    let canonical = content["website"]["canonical"].as_str().unwrap_or("");
    let path = "/blog/";
    let page = |fragment: &str| page_id(canonical, path, fragment);

    let article_items: Vec<Value> = articles
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let article_url = format!("{canonical}{}", p.url());
            json!({
                "@type": "ListItem",
                "position": i + 1,
                "item": {
                    "@type": "WebPage",
                    "@id": format!("{article_url}#page"),
                    "url": format!("{article_url}#page"),
                    "name": p.frontmatter.title
                }
            })
        })
        .collect();

    json!({
        "@context": "https://schema.org",
        "@graph": [
            person_node(content),
            website_node(content),
            json!({
                "@type": ["Blog", "WebPage"],
                "@id": page("page"),
                "url": page("page"),
                "name": label(content, "blog"),
                "description": description,
                "inLanguage": site_lang(content),
                "isPartOf": { "@id": root_id(canonical, "web") },
                "breadcrumb": { "@id": page("breadcrumb") },
                "mainEntity": { "@id": page("articles") },
                "publisher": { "@id": root_id(canonical, crate::routes::SLICE_PERSON) }
            }),
            json!({
                "@type": "ItemList",
                "@id": page("articles"),
                "url": page("articles"),
                "name": label(content, "articles"),
                "numberOfItems": article_items.len(),
                "itemListElement": article_items
            }),
            json!({
                "@type": "BreadcrumbList",
                "@id": page("breadcrumb"),
                "url": page("breadcrumb"),
                "itemListElement": [
                    {
                        "@type": "ListItem",
                        "position": 1,
                        "item": { "@type": "WebPage", "@id": canonical, "url": canonical, "name": label(content, "home") }
                    },
                    {
                        "@type": "ListItem",
                        "position": 2,
                        "item": {
                            "@type": "WebPage",
                            "@id": page("articles"),
                            "url": page("articles"),
                            "name": label(content, "blog")
                        }
                    }
                ]
            })
        ]
    })
}

pub fn tag_graph(content: &Value, tag: &str, slug: &str, tagged: &[&Article], description: &str) -> Value {
    let canonical = content["website"]["canonical"].as_str().unwrap_or("");
    let display = tag_display(content, tag);
    let path = format!("/blog/tags/{slug}/");
    let page = |fragment: &str| page_id(canonical, &path, fragment);

    let article_items: Vec<Value> = tagged
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let article_url = format!("{canonical}{}", p.url());
            json!({
                "@type": "ListItem",
                "position": i + 1,
                "item": {
                    "@type": "WebPage",
                    "@id": format!("{article_url}#page"),
                    "url": format!("{article_url}#page"),
                    "name": p.frontmatter.title
                }
            })
        })
        .collect();

    json!({
        "@context": "https://schema.org",
        "@graph": [
            person_node(content),
            website_node(content),
            json!({
                "@type": ["CollectionPage", "WebPage"],
                "@id": page("page"),
                "url": page("page"),
                "name": display,
                "description": description,
                "inLanguage": site_lang(content),
                "isPartOf": { "@id": root_id(canonical, "web") },
                "breadcrumb": { "@id": page("breadcrumb") },
                "mainEntity": { "@id": page("articles") },
                "publisher": { "@id": root_id(canonical, crate::routes::SLICE_PERSON) }
            }),
            json!({
                "@type": "ItemList",
                "@id": page("articles"),
                "url": page("articles"),
                "name": display,
                "numberOfItems": article_items.len(),
                "itemListElement": article_items
            }),
            json!({
                "@type": "BreadcrumbList",
                "@id": page("breadcrumb"),
                "url": page("breadcrumb"),
                "itemListElement": [
                    {
                        "@type": "ListItem",
                        "position": 1,
                        "item": { "@type": "WebPage", "@id": canonical, "url": canonical, "name": label(content, "home") }
                    },
                    {
                        "@type": "ListItem",
                        "position": 2,
                        "item": {
                            "@type": "WebPage",
                            "@id": blog_url(canonical),
                            "url": blog_url(canonical),
                            "name": label(content, "blog")
                        }
                    },
                    {
                        "@type": "ListItem",
                        "position": 3,
                        "item": {
                            "@type": "WebPage",
                            "@id": format!("{canonical}/blog/tags/"),
                            "url": format!("{canonical}/blog/tags/"),
                            "name": label(content, "tags")
                        }
                    },
                    {
                        "@type": "ListItem",
                        "position": 4,
                        "item": {
                            "@type": "WebPage",
                            "@id": page("articles"),
                            "url": page("articles"),
                            "name": display
                        }
                    }
                ]
            })
        ]
    })
}

pub fn tags_index_graph(
    content: &Value,
    tags: &[(&str, String, Vec<&Article>)],
    description: &str,
) -> Value {
    let canonical = content["website"]["canonical"].as_str().unwrap_or("");
    let path = "/blog/tags/";
    let page = |fragment: &str| page_id(canonical, path, fragment);

    let tag_items: Vec<Value> = tags
        .iter()
        .enumerate()
        .map(|(i, (tag, slug, _tagged))| {
            json!({
                "@type": "ListItem",
                "position": i + 1,
                "item": {
                    "@type": "WebPage",
                    "@id": format!("{canonical}/blog/tags/{slug}/#page"),
                    "url": format!("{canonical}/blog/tags/{slug}/#page"),
                    "name": tag_display(content, tag)
                }
            })
        })
        .collect();

    json!({
        "@context": "https://schema.org",
        "@graph": [
            person_node(content),
            website_node(content),
            json!({
                "@type": ["CollectionPage", "WebPage"],
                "@id": page("page"),
                "url": page("page"),
                "name": label(content, "tags"),
                "description": description,
                "inLanguage": site_lang(content),
                "isPartOf": { "@id": root_id(canonical, "web") },
                "breadcrumb": { "@id": page("breadcrumb") },
                "mainEntity": { "@id": page("tags") },
                "publisher": { "@id": root_id(canonical, crate::routes::SLICE_PERSON) }
            }),
            json!({
                "@type": "ItemList",
                "@id": page("tags"),
                "url": page("tags"),
                "name": label(content, "tags"),
                "numberOfItems": tag_items.len(),
                "itemListElement": tag_items
            }),
            json!({
                "@type": "BreadcrumbList",
                "@id": page("breadcrumb"),
                "url": page("breadcrumb"),
                "itemListElement": [
                    {
                        "@type": "ListItem",
                        "position": 1,
                        "item": { "@type": "WebPage", "@id": canonical, "url": canonical, "name": label(content, "home") }
                    },
                    {
                        "@type": "ListItem",
                        "position": 2,
                        "item": {
                            "@type": "WebPage",
                            "@id": blog_url(canonical),
                            "url": blog_url(canonical),
                            "name": label(content, "blog")
                        }
                    },
                    {
                        "@type": "ListItem",
                        "position": 3,
                        "item": {
                            "@type": "WebPage",
                            "@id": page("tags"),
                            "url": page("tags"),
                            "name": label(content, "tags")
                        }
                    }
                ]
            })
        ]
    })
}

#[derive(Debug)]
pub struct AuditIssue {
    pub node_type: String,
    pub id: Option<String>,
    pub message: String,
}

impl AuditIssue {
    fn new(node_type: &str, id: Option<String>, message: impl Into<String>) -> Self {
        Self { node_type: node_type.to_owned(), id, message: message.into() }
    }
}

fn types_of(node: &Value) -> Vec<String> {
    match &node["@type"] {
        Value::Array(types) => types
            .iter()
            .filter_map(|t| t.as_str().map(str::to_owned))
            .collect(),
        Value::String(t) => vec![t.clone()],
        _ => Vec::new(),
    }
}

fn flatten(value: &Value, out: &mut Vec<Value>) {
    match value {
        Value::Array(items) => items.iter().for_each(|item| flatten(item, out)),
        Value::Object(map) => {
            let node = Value::Object(map.clone());
            if !types_of(&node).is_empty() {
                out.push(node.clone());
            }
            map.values().for_each(|item| flatten(item, out));
        }
        _ => {}
    }
}

fn element_ids(html: &str) -> std::collections::HashMap<String, usize> {
    let mut ids = std::collections::HashMap::new();
    let mut rest = html;
    while let Some(pos) = rest.find("id=") {
        let after = &rest[pos + 3..];
        let value = if let Some(stripped) = after.strip_prefix('"') {
            stripped.split('"').next()
        } else if let Some(stripped) = after.strip_prefix('\'') {
            stripped.split('\'').next()
        } else {
            after.split(|c: char| c.is_whitespace() || c == '>' || c == '/').next()
        };
        if let Some(id) = value.filter(|s| !s.is_empty()) {
            *ids.entry(id.to_owned()).or_insert(0) += 1;
        }
        rest = &rest[pos + 3..];
    }
    ids
}

fn collect_graph(html: &str) -> Result<Vec<Value>, Box<dyn Error>> {
    let mut nodes = Vec::new();
    let mut rest = html;
    while let Some(start) = rest.find("<script") {
        let tag_end = rest[start..]
            .find('>')
            .map(|i| start + i + 1)
            .ok_or("unterminated script tag")?;
        let tag = &rest[start..tag_end];
        rest = &rest[tag_end..];
        if !tag.contains("application/ld+json") {
            continue;
        }
        let end = rest
            .find("</script>")
            .ok_or("unterminated ld+json script block")?;
        let parsed: Value = serde_json::from_str(rest[..end].trim())
            .map_err(|e| format!("ld+json block does not parse: {e}"))?;
        flatten(&parsed, &mut nodes);
        rest = &rest[end + "</script>".len()..];
    }
    Ok(nodes)
}

fn fragment_of(value: &str) -> Option<&str> {
    value.find('#').map(|i| &value[i + 1..])
}

fn parse_attrs(tag: &str) -> Vec<(String, String)> {
    let body = tag.trim_start_matches('<').trim_end_matches('>');
    let tag_name_len = body.find(char::is_whitespace).unwrap_or(body.len());
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    for c in body[tag_name_len..].chars() {
        match quote {
            Some(q) if c == q => {
                quote = None;
                current.push(c);
            }
            _ if c == '"' || c == '\'' => {
                quote = Some(c);
                current.push(c);
            }
            _ if c.is_whitespace() && quote.is_none() => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(c),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
        .into_iter()
        .filter_map(|token| {
            let (key, value) = token.split_once('=')?;
            let value = value
                .trim_matches('"')
                .trim_matches('\'')
                .to_owned();
            Some((key.to_owned(), value))
        })
        .collect()
}

fn tag_at(html: &str, id: &str) -> Option<(usize, usize)> {
    let forms = [format!("id={id}"), format!("id=\"{id}\""), format!("id='{id}'")];
    for needle in &forms {
        let mut rest = html;
        let mut offset = 0;
        while let Some(pos) = rest.find(needle.as_str()) {
            let after = rest[pos + needle.len()..].chars().next();
            let boundary = matches!(after, Some('>') | Some('/') | Some(' ') | Some('"') | Some('\'') | None);
            if boundary {
                let at = offset + pos;
                let start = html[..at].rfind('<')?;
                let end = at + html[at..].find('>')?;
                return Some((start, end));
            }
            offset += pos + needle.len();
            rest = &rest[pos + needle.len()..];
        }
    }
    None
}

fn attr_of(html: &str, id: &str, attr: &str) -> Option<String> {
    let (start, end) = tag_at(html, id)?;
    let tag = &html[start..=end];
    parse_attrs(tag)
        .into_iter()
        .find(|(key, _)| key == attr)
        .map(|(_, value)| value)
}

fn text_of(html: &str, id: &str) -> Option<String> {
    let (start, end) = tag_at(html, id)?;
    let tag = &html[start..=end];
    let name_end = tag[1..]
        .find(|c: char| c.is_whitespace() || c == '>')
        .map(|i| i + 1)
        .unwrap_or(tag.len() - 1);
    let name = &tag[1..name_end];
    let after = &html[end + 1..];
    let close = after.find(&format!("</{name}>"))?;
    let inner = &after[..close];
    let mut text = String::new();
    let mut rest = inner;
    while let Some(pos) = rest.find('<') {
        text.push_str(&rest[..pos]);
        let tag_end = rest[pos..].find('>')? + pos;
        rest = &rest[tag_end + 1..];
    }
    text.push_str(rest);
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    (!collapsed.is_empty()).then_some(collapsed)
}

fn accessible_name(html: &str, id: &str) -> Option<String> {
    if let Some(referenced) = attr_of(html, id, "aria-labelledby") {
        let parts: Vec<String> = referenced
            .split_whitespace()
            .filter_map(|ref_id| text_of(html, ref_id))
            .collect();
        if !parts.is_empty() {
            return Some(parts.join(" "));
        }
    }
    attr_of(html, id, "aria-label")
}

fn meta_contents(html: &str) -> Vec<(String, String, String)> {
    let mut metas = Vec::new();
    let mut rest = html;
    let mut offset = 0;
    while let Some(pos) = rest.find("<meta") {
        let at = offset + pos;
        let Some(end) = html[at..].find('>').map(|i| at + i) else { break };
        let attrs = parse_attrs(&html[at..=end]);
        let key = attrs
            .iter()
            .find(|(k, _)| k == "name" || k == "property")
            .map(|(_, v)| v.clone());
        let content = attrs
            .iter()
            .find(|(k, _)| k == "content")
            .map(|(_, v)| v.clone());
        if let (Some(key), Some(content)) = (key, content) {
            metas.push((key, content, html[at..=end].to_owned()));
        }
        offset = end;
        rest = &html[end..];
    }
    metas
}

fn list_item_count(html: &str, id: &str) -> Option<usize> {
    let (_, end) = tag_at(html, id)?;
    let after = &html[end + 1..];
    let close = after.find("</ul>").or_else(|| after.find("</ol>"))?;
    let span = &after[..close];
    if span.contains("<ul") || span.contains("<ol") {
        return None;
    }
    let mut count = 0;
    let mut rest = span;
    while let Some(pos) = rest.find("<li") {
        let next_char = rest[pos + 3..].chars().next();
        if matches!(next_char, Some(' ') | Some('>') | Some('/')) {
            count += 1;
        }
        rest = &rest[pos + 3..];
    }
    Some(count)
}

fn path_of(value: &str) -> &str {
    match value.find('#') {
        Some(i) => &value[..i],
        None => value,
    }
}

fn same_origin(value: &str, canonical: &str) -> bool {
    value.starts_with(canonical)
}

fn anchor_hrefs(html: &str) -> Vec<String> {
    let mut hrefs = Vec::new();
    let mut rest = html;
    let mut offset = 0;
    while let Some(pos) = rest.find("<a ") {
        let at = offset + pos;
        let Some(end) = html[at..].find('>').map(|i| at + i) else { break };
        if let Some((_, href)) = parse_attrs(&html[at..=end])
            .into_iter()
            .find(|(key, _)| key == "href")
        {
            hrefs.push(href);
        }
        offset = end;
        rest = &html[end..];
    }
    hrefs
}

const FILE_EXTENSION_RE: &str = ".txt.xml.md.png.ico.svg.webmanifest.jpg.jpeg.webp.css.js.json";

fn is_file_path(path: &str) -> bool {
    path.rsplit_once('.')
        .map(|(_, ext)| FILE_EXTENSION_RE.contains(&format!(".{ext}.")))
        .unwrap_or(false)
}

pub fn audit_page(canonical: &str, page_path: &str, html: &str) -> Result<(), Vec<AuditIssue>> {
    let nodes = match collect_graph(html) {
        Ok(nodes) => nodes,
        Err(e) => return Err(vec![AuditIssue::new("document", None, e.to_string())]),
    };
    let ids = element_ids(html);

    let defined: Vec<Option<String>> = nodes
        .iter()
        .map(|n| n["@id"].as_str().map(str::to_owned))
        .collect();
    let this_page = format!("{canonical}{page_path}");
    let same_page = |url: &str| {
        path_of(url).trim_end_matches('/') == this_page.trim_end_matches('/')
    };

    let mut issues = Vec::new();
    let is_defined = |id: &str| defined.iter().any(|d| d.as_deref() == Some(id));

    for href in anchor_hrefs(html) {
        if href.starts_with('#') {
            let fragment = &href[1..];
            if !fragment.is_empty() && !ids.contains_key(fragment) {
                issues.push(AuditIssue::new(
                    "link",
                    Some(href.clone()),
                    format!("same-page link \"{href}\" resolves to no element with id=\"{fragment}\""),
                ));
            }
            continue;
        }
        if !href.starts_with('/') || href.starts_with("//") {
            continue;
        }
        let path = href.split('#').next().unwrap_or("");
        if is_file_path(path) {
            continue;
        }
        if !href.contains('#') {
            issues.push(AuditIssue::new(
                "link",
                Some(href.clone()),
                format!("internal link \"{href}\" carries no fragment (expected #page style targets)"),
            ));
        }
    }

    for node in &nodes {
        let types = types_of(node);
        let type_label = types.join(",");
        let node_id = node["@id"].as_str().map(str::to_owned);

        if types.iter().any(|t| SUPERIOR_TYPES.contains(&t.as_str())) {
            if node["name"].is_null() {
                issues.push(AuditIssue::new(&type_label, node_id.clone(), "missing name"));
            }
            if node_id.is_none() {
                issues.push(AuditIssue::new(&type_label, None, "missing @id"));
            }
            continue;
        }
        if !types.iter().any(|t| SUBORDINATE_TYPES.contains(&t.as_str())) {
            continue;
        }

        let mut id = node_id.clone();
        let mut url = node["url"].as_str().map(str::to_owned);
        if types.iter().any(|t| t == "ListItem") {
            if let Some(item) = node.get("item").filter(|i| i.is_object()) {
                id = item["@id"].as_str().map(str::to_owned).or(id);
                url = item["url"].as_str().map(str::to_owned).or(url);
            } else if let Some(item) = node["item"].as_str() {
                id = Some(item.to_owned());
                url = None;
            }
        }

        let (Some(id), Some(url)) = (id, url) else {
            issues.push(AuditIssue::new(&type_label, node_id, "missing @id or url"));
            continue;
        };
        if id != url {
            issues.push(AuditIssue::new(
                &type_label,
                Some(id.clone()),
                format!("@id \"{id}\" does not match url \"{url}\""),
            ));
            continue;
        }
        if !same_origin(&url, canonical) {
            continue;
        }
        if !same_page(&url) {
            continue;
        }
        if fragment_of(&url).is_some() && path_of(&url) == canonical && !canonical.ends_with('/') {
            issues.push(AuditIssue::new(
                &type_label,
                Some(id.clone()),
                format!("root url \"{url}\" must end with / before the #fragment"),
            ));
            continue;
        }
        let Some(fragment) = fragment_of(&url) else {
            issues.push(AuditIssue::new(
                &type_label,
                Some(id.clone()),
                format!("url \"{url}\" has no #identifier"),
            ));
            continue;
        };
        if !ids.contains_key(fragment) {
            issues.push(AuditIssue::new(
                &type_label,
                Some(id.clone()),
                format!("no element with id=\"{fragment}\""),
            ));
        }
    }

    fn collect_refs(value: &Value, out: &mut Vec<String>) {
        match value {
            Value::Array(items) => items.iter().for_each(|i| collect_refs(i, out)),
            Value::Object(map) => {
                for (key, item) in map {
                    if key == "@id" {
                        if let Some(id) = item.as_str() {
                            out.push(id.to_owned());
                        }
                    } else {
                        collect_refs(item, out);
                    }
                }
            }
            _ => {}
        }
    }
    let mut refs = Vec::new();
    collect_refs(&Value::Array(nodes.clone()), &mut refs);
    for reference in refs {
        if !same_origin(&reference, canonical) {
            continue;
        }
        let Some(_) = fragment_of(&reference) else { continue };
        let root = path_of(&reference).trim_end_matches('/') == canonical.trim_end_matches('/');
        if (root || same_page(&reference)) && !is_defined(&reference) {
            issues.push(AuditIssue::new(
                "reference",
                Some(reference.clone()),
                format!("reference \"{reference}\" is not defined in the page graph"),
            ));
        }
    }

    for node in &nodes {
        let types = types_of(node);
        if !types.iter().any(|t| LIST_TYPES.contains(&t.as_str())) {
            continue;
        }
        let type_label = types.join(",");
        let node_id = node["@id"].as_str().map(str::to_owned);
        let Some(items) = node["itemListElement"].as_array() else { continue };
        for (index, entry) in items.iter().enumerate() {
            let position = entry["position"].as_u64();
            if position != Some(index as u64 + 1) {
                issues.push(AuditIssue::new(
                    &type_label,
                    node_id.clone(),
                    format!("itemListElement[{index}] position is not the sequential {}", index + 1),
                ));
            }
        }
    }

    for node in &nodes {
        let types = types_of(node);
        if !types.iter().any(|t| PAGE_TYPES.contains(&t.as_str())) {
            continue;
        }
        let Some(main_id) = node["mainEntity"]["@id"].as_str() else { continue };
        let type_label = types.join(",");
        let node_id = node["@id"].as_str().map(str::to_owned);
        let Some(main_fragment) = fragment_of(main_id) else {
            issues.push(AuditIssue::new(
                &type_label,
                node_id,
                format!("mainEntity \"{main_id}\" is missing a #fragment"),
            ));
            continue;
        };
        for breadcrumb in nodes.iter().filter(|n| types_of(n).iter().any(|t| t == "BreadcrumbList")) {
            let Some(items) = breadcrumb["itemListElement"].as_array() else { continue };
            let Some(last) = items.last() else { continue };
            let last_url = last["item"]["url"].as_str().or_else(|| last["item"].as_str());
            let Some(last_url) = last_url else { continue };
            if fragment_of(last_url) != Some(main_fragment) {
                issues.push(AuditIssue::new(
                    "BreadcrumbList",
                    breadcrumb["@id"].as_str().map(str::to_owned),
                    format!(
                        "last breadcrumb item \"{last_url}\" does not match the mainEntity anchor #{main_fragment}"
                    ),
                ));
            }
        }
    }

    let metas = meta_contents(html);
    let meta_value = |key: &str| -> Option<String> {
        metas
            .iter()
            .find(|(k, _, _)| k == key)
            .map(|(_, content, _)| content.clone())
    };

    for node in &nodes {
        let types = types_of(node);
        if !types.iter().any(|t| PAGE_TYPES.contains(&t.as_str())) {
            continue;
        }
        let type_label = types.join(",");
        let node_id = node["@id"].as_str().map(str::to_owned);

        if let Some(lang) = node["inLanguage"].as_str() {
            if let Some(html_lang) = attr_of(html, "web", "lang").or_else(|| {
                html.split('>')
                    .next()
                    .and_then(|first| parse_attrs(first).into_iter().find(|(k, _)| k == "lang"))
                    .map(|(_, v)| v)
            }) {
                if html_lang != lang {
                    issues.push(AuditIssue::new(
                        &type_label,
                        node_id.clone(),
                        format!("inLanguage \"{lang}\" does not match the html lang \"{html_lang}\""),
                    ));
                }
                if let Some(og) = meta_value("og:locale") {
                    if !(og == lang || og.starts_with(&format!("{lang}_"))) {
                        issues.push(AuditIssue::new(
                            &type_label,
                            node_id.clone(),
                            format!("og:locale \"{og}\" does not agree with inLanguage \"{lang}\""),
                        ));
                    }
                }
            }
        }

        if let Some(description) = node["description"].as_str() {
            for key in ["description", "og:description"] {
                match meta_value(key) {
                    Some(meta) if meta == description => {}
                    Some(meta) => issues.push(AuditIssue::new(
                        &type_label,
                        node_id.clone(),
                        format!("{key} meta \"{meta}\" does not match the graph description"),
                    )),
                    None => issues.push(AuditIssue::new(
                        &type_label,
                        node_id.clone(),
                        format!("{key} meta is missing while the graph carries a description"),
                    )),
                }
            }
        }
    }

    for node in &nodes {
        let types = types_of(node);
        if !types.iter().any(|t| t == "BlogPosting") {
            continue;
        }
        let type_label = types.join(",");
        let node_id = node["@id"].as_str().map(str::to_owned);
        if let Some(keywords) = node["keywords"].as_str() {
            let mut expected: Vec<&str> = keywords.split(',').map(str::trim).collect();
            expected.sort_unstable();
            let mut actual: Vec<String> = metas
                .iter()
                .filter(|(k, _, _)| k == "article:tag")
                .map(|(_, content, _)| content.trim().to_owned())
                .collect();
            actual.sort();
            if expected != actual {
                issues.push(AuditIssue::new(
                    &type_label,
                    node_id,
                    format!("article:tag metas {actual:?} do not match the keywords entries {expected:?}"),
                ));
            }
        }
    }

    for node in &nodes {
        let types = types_of(node);
        if !types.iter().any(|t| t == "ItemList") {
            continue;
        }
        let type_label = types.join(",");
        let node_id = node["@id"].as_str().map(str::to_owned);
        let Some(url) = node["url"].as_str() else { continue };
        if !same_page(url) {
            continue;
        }
        let Some(fragment) = fragment_of(url) else { continue };

        if let Some(name) = node["name"].as_str() {
            match accessible_name(html, fragment) {
                Some(label) if label == name => {}
                Some(label) => issues.push(AuditIssue::new(
                    &type_label,
                    node_id.clone(),
                    format!("ItemList name \"{name}\" does not match the accessible name \"{label}\" of #{fragment}"),
                )),
                None => issues.push(AuditIssue::new(
                    &type_label,
                    node_id.clone(),
                    format!("ItemList name \"{name}\" has no accessible name (aria-label or aria-labelledby) on #{fragment}"),
                )),
            }
        }

        if let (Some(count), Some(items)) = (
            list_item_count(html, fragment),
            node["numberOfItems"].as_u64(),
        ) {
            if count as u64 != items {
                issues.push(AuditIssue::new(
                    &type_label,
                    node_id,
                    format!("numberOfItems {items} does not match the {count} li children of #{fragment}"),
                ));
            }
        }
    }

    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}

pub fn audit_report(page_path: &str, issues: &[AuditIssue]) -> String {
    issues
        .iter()
        .map(|i| {
            format!(
                "schema audit failed for {page_path}: {} {}: {}",
                i.node_type,
                i.id.as_deref().unwrap_or("<no id>"),
                i.message
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const CANONICAL: &str = "https://example.com";

    fn page_id() -> String {
        format!("{CANONICAL}/a/#page")
    }

    fn article_id() -> String {
        format!("{CANONICAL}/a/#article")
    }

    fn crumbs_id() -> String {
        format!("{CANONICAL}/a/#crumbs")
    }

    fn person_id() -> String {
        format!("{CANONICAL}/#person")
    }

    fn web_id() -> String {
        format!("{CANONICAL}/#web")
    }

    #[test]
    fn collects_ld_json_blocks_and_ids() {
        let html = format!(
            r#"<script type="application/ld+json">{{"@context":"https://schema.org","@graph":[{{"@type":"WebPage","@id":"{}","url":"{}"}}]}}</script><main id=page></main>"#,
            page_id(),
            page_id()
        );
        let nodes = collect_graph(&html).unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(element_ids(&html).get("page"), Some(&1));
    }

    fn label_content() -> Value {
        serde_json::json!({
            "person": {
                "name": "Someone",
                "email": "a@b.c",
                "description": "Person desc",
                "links": [ { "title": "Github", "href": "https://github.com/x" } ]
            },
            "website": {
                "canonical": CANONICAL,
                "lang": "de",
                "note": "Note text",
                "tags": { "graphtheory": "Graph theory" },
                "titles": { "home": "Start" },
                "descriptions": { "home": "Home desc" },
                "labels": {
                    "home": "Start", "blog": "Journal", "tags": "Topics",
                    "links": "Connections", "recent": "Fresh writing", "articles": "Entries",
                    "toc": "On this page",
                    "toc_list": "Table of contents"
                }
            }
        })
    }

    #[test]
    fn graph_names_follow_content_labels_and_lang() {
        let content = label_content();

        let home = home_graph(&content, &[]).to_string();
        assert!(home.contains("\"name\":\"Connections\""));
        assert!(!home.contains("Contacts"));
        assert!(home.contains("\"inLanguage\":\"de\""));

        let blog = blog_index_graph(&content, &[], "Blog desc").to_string();
        assert!(blog.contains("\"name\":\"Journal\""));
        assert!(blog.contains("\"name\":\"Entries\""));
        assert!(blog.contains("\"name\":\"Start\""));

        let tag = tag_graph(&content, "graphtheory", "graphtheory", &[], "Tag desc").to_string();
        assert!(tag.contains("\"name\":\"Graph theory\""));
        assert!(tag.contains("\"name\":\"Graph theory\""));
        assert!(!tag.contains("Entries tagged"));
        assert!(!tag.contains("#Graph theory"));

        let hub = tags_index_graph(&content, &[], "Hub desc").to_string();
        assert!(hub.contains("\"name\":\"Topics\""));
    }

    #[test]
    fn audit_accepts_a_coherent_article_graph() {
        let graph = serde_json::json!({
            "@context": "https://schema.org",
            "@graph": [
                { "@type": "WebPage", "@id": page_id(), "url": page_id(),
                  "mainEntity": { "@id": article_id() },
                  "breadcrumb": { "@id": crumbs_id() } },
                { "@type": "BlogPosting", "@id": article_id(), "url": article_id(),
                  "author": { "@id": person_id() } },
                { "@type": "Person", "@id": person_id(), "url": person_id(), "name": "Someone" },
                { "@type": "WebSite", "@id": web_id(), "url": CANONICAL, "name": "Site" },
                { "@type": "BreadcrumbList", "@id": crumbs_id(), "url": crumbs_id(),
                  "itemListElement": [
                    { "@type": "ListItem", "position": 1, "item": { "@id": CANONICAL, "url": CANONICAL, "name": "Home" } },
                    { "@type": "ListItem", "position": 2, "item": { "@id": article_id(), "url": article_id(), "name": "A" } }
                  ] }
            ]
        });
        let html = format!(
            r#"<html><main id=page tabindex=-1><article id=article></article><nav id=toc aria-label="Table of contents"><ul id=toc-list><li id=intro><a href=#page>Intro</a></li></ul></nav><nav id=crumbs aria-label=Breadcrumb></nav><section id=person>Someone</section><script type="application/ld+json">{graph}</script></html>"#
        );
        match audit_page(CANONICAL, "/a/", &html) {
            Ok(()) => {}
            Err(issues) => panic!("coherent graph must pass: {issues:?}"),
        }
    }

    #[test]
    fn audit_flags_dangling_reference() {
        let graph = serde_json::json!({
            "@graph": [
                { "@type": "BlogPosting", "@id": article_id(), "url": article_id(),
                  "author": { "@id": person_id() } }
            ]
        });
        let html = format!(
            r#"<main id=page><article id=article></article><script type="application/ld+json">{graph}</script>"#
        );
        let issues = audit_page(CANONICAL, "/a/", &html).unwrap_err();
        assert!(issues.iter().any(|i| i.message.contains("not defined")), "{issues:?}");
    }

    #[test]
    fn audit_flags_ungrounded_fragment() {
        let missing = format!("{CANONICAL}/a/#missing");
        let graph = serde_json::json!({
            "@type": "WebPage", "@id": missing, "url": missing
        });
        let html = format!(
            r#"<main id=page><script type="application/ld+json">{graph}</script>"#
        );
        let issues = audit_page(CANONICAL, "/a/", &html).unwrap_err();
        assert!(issues.iter().any(|i| i.message.contains("no element with id")), "{issues:?}");
    }

    #[test]
    fn audit_flags_id_url_mismatch_and_positions() {
        let other = format!("{CANONICAL}/a/#other");
        let graph = serde_json::json!({
            "@type": "BreadcrumbList", "@id": crumbs_id(), "url": other,
            "itemListElement": [ { "@type": "ListItem", "position": 2 } ]
        });
        let html = format!(
            r#"<main id=page><nav id=crumbs></nav><script type="application/ld+json">{graph}</script>"#
        );
        let issues = audit_page(CANONICAL, "/a/", &html).unwrap_err();
        assert!(issues.iter().any(|i| i.message.contains("does not match")), "{issues:?}");
        assert!(issues.iter().any(|i| i.message.contains("not the sequential")), "{issues:?}");
    }

    #[test]
    fn audit_flags_clean_internal_links_and_dangling_self_fragments() {
        let html = r##"<main id=page><a href="/blog/">clean</a><a href="#missing">self</a><a href="/llms.txt">file</a><a href="https://example.com/external/">ext</a></main>"##;
        let issues = audit_page(CANONICAL, "/a/", html).unwrap_err();
        assert!(
            issues
                .iter()
                .any(|i| i.message.contains("internal link \"/blog/\" carries no fragment")),
            "{issues:?}"
        );
        assert!(
            issues
                .iter()
                .any(|i| i.message.contains("same-page link \"#missing\" resolves to no element")),
            "{issues:?}"
        );
        assert!(
            !issues.iter().any(|i| i.message.contains("/llms.txt")),
            "{issues:?}"
        );
        assert!(
            !issues.iter().any(|i| i.message.contains("external")),
            "{issues:?}"
        );
    }

    #[test]
    fn audit_flags_root_fragment_without_slash() {
        let bare_root = format!("{CANONICAL}#page");
        let graph = serde_json::json!({
            "@type": "WebPage", "@id": bare_root, "url": bare_root
        });
        let html = format!(
            r#"<main id=page><script type="application/ld+json">{graph}</script>"#
        );
        let issues = audit_page(CANONICAL, "/", &html).unwrap_err();
        assert!(
            issues
                .iter()
                .any(|i| i.message.contains("must end with / before the #fragment")),
            "{issues:?}"
        );
    }

    #[test]
    fn audit_flags_meta_description_divergence() {
        let graph = serde_json::json!({
            "@graph": [
                { "@type": "WebPage", "@id": page_id(), "url": page_id(),
                  "description": "Graph description",
                  "mainEntity": { "@id": article_id() },
                  "breadcrumb": { "@id": crumbs_id() } },
                { "@type": "BreadcrumbList", "@id": crumbs_id(), "url": crumbs_id(),
                  "itemListElement": [
                    { "@type": "ListItem", "position": 1, "item": { "@id": CANONICAL, "url": CANONICAL, "name": "Home" } },
                    { "@type": "ListItem", "position": 2, "item": { "@id": article_id(), "url": article_id(), "name": "A" } }
                  ] }
            ]
        });
        let html = format!(
            r#"<html><main id=page><article id=article></article><nav id=crumbs></nav><meta name=description content=Different><meta property=og:description content="Graph description"><script type="application/ld+json">{graph}</script></html>"#
        );
        let issues = audit_page(CANONICAL, "/a/", &html).unwrap_err();
        assert!(
            issues
                .iter()
                .any(|i| i.message.contains("description meta \"Different\" does not match the graph description")),
            "{issues:?}"
        );
    }

    #[test]
    fn audit_flags_list_label_and_count_divergence() {
        let list_id = format!("{CANONICAL}/a/#articles");
        let graph = serde_json::json!({
            "@type": "ItemList", "@id": list_id, "url": list_id,
            "name": "Articles", "numberOfItems": 3
        });
        let html = format!(
            r#"<main id=page><ul id=articles aria-label=Wrong><li>one</li></ul><script type="application/ld+json">{graph}</script>"#
        );
        let issues = audit_page(CANONICAL, "/a/", &html).unwrap_err();
        assert!(
            issues
                .iter()
                .any(|i| i.message.contains("does not match the accessible name \"Wrong\"")),
            "{issues:?}"
        );
        assert!(
            issues
                .iter()
                .any(|i| i.message.contains("numberOfItems 3 does not match the 1 li children")),
            "{issues:?}"
        );
    }

    #[test]
    fn audit_resolves_list_names_through_aria_labelledby() {
        let list_id = format!("{CANONICAL}/a/#articles");
        let graph = serde_json::json!({
            "@graph": [
                { "@type": "ItemList", "@id": list_id, "url": list_id,
                  "name": "Graph theory", "numberOfItems": 1 }
            ]
        });
        let html = format!(
            r#"<main id=page><h1 id=tag-title>Graph theory</h1><ul id=articles aria-labelledby="tag-title"><li>one</li></ul><script type="application/ld+json">{graph}</script>"#
        );
        assert!(
            audit_page(CANONICAL, "/a/", &html).is_ok(),
            "labelledby-resolved name must satisfy the ItemList check"
        );

        let graph = serde_json::json!({
            "@graph": [
                { "@type": "ItemList", "@id": list_id, "url": list_id,
                  "name": "Something else", "numberOfItems": 1 }
            ]
        });
        let html = format!(
            r#"<main id=page><h1 id=tag-title>Graph theory</h1><ul id=articles aria-labelledby="tag-title"><li>one</li></ul><script type="application/ld+json">{graph}</script>"#
        );
        let issues = audit_page(CANONICAL, "/a/", &html).unwrap_err();
        assert!(
            issues
                .iter()
                .any(|i| i.message.contains("does not match the accessible name \"Graph theory\"")),
            "{issues:?}"
        );
    }

    #[test]
    fn audit_flags_breadcrumb_main_entity_mismatch() {
        let graph = serde_json::json!({
            "@graph": [
                { "@type": "WebPage", "@id": page_id(), "url": page_id(),
                  "mainEntity": { "@id": article_id() } },
                { "@type": "BreadcrumbList", "@id": crumbs_id(), "url": crumbs_id(),
                  "itemListElement": [
                    { "@type": "ListItem", "position": 1, "item": { "@id": CANONICAL, "url": CANONICAL, "name": "Home" } },
                    { "@type": "ListItem", "position": 2, "item": { "@id": page_id(), "url": page_id(), "name": "A" } }
                  ] }
            ]
        });
        let html = format!(
            r#"<main id=page><article id=article></article><nav id=crumbs></nav><script type="application/ld+json">{graph}</script>"#
        );
        let issues = audit_page(CANONICAL, "/a/", &html).unwrap_err();
        assert!(
            issues.iter().any(|i| i.message.contains("does not match the mainEntity anchor")),
            "{issues:?}"
        );
    }
}
