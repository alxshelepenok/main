use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    sync::LazyLock,
};

mod markdown;
mod mermaid;
mod og;
mod articles;
mod routes;
mod schema;

use tera::{Context, Tera};

const INPUT_PATH: &str = "src";
const OUTPUT_PATH: &str = "target";
const INDEX_OUTPUT_FILE: &str = "index.html";
const INDEX_TEMPLATE_FILE: &str = "index.hbs";
const NOT_FOUND_OUTPUT_FILE: &str = "404.html";
const NOT_FOUND_TEMPLATE_FILE: &str = "404.hbs";
const ARTICLE_TEMPLATE_FILE: &str = "article.hbs";
const ARTICLES_INPUT_DIR: &str = "articles";
const RECENT_ARTICLES_COUNT: usize = 5;
const ATOM_TEMPLATE_FILE: &str = "atom.hbs";
const ATOM_OUTPUT_FILE: &str = "atom.xml";
const TAG_TEMPLATE_FILE: &str = "tag.hbs";
const TAGS_TEMPLATE_FILE: &str = "tags.hbs";
const BLOG_TEMPLATE_FILE: &str = "blog.hbs";
const BLOG_INDEX_OUTPUT_FILE: &str = "blog/index.html";
const OG_BACKGROUND_FILE: &str = "bg.png";
const CONTENT_FILE: &str = "content.json";
const TEMPLATE_PATH: &str = "src/**/*.hbs";
const STYLE_FILE: &str = "style.css";
const ROBOTS_INDEX: &str = "index, follow, max-image-preview:large, max-snippet:140001000";
const ROBOTS_NOINDEX: &str = "noindex, follow";
const COPY_FILES: [&str; 9] = [
    "_headers",
    "robots.txt",
    "alxshelepenok.webmanifest",
    "apple-touch-icon.png",
    "favicon.ico",
    "favicon.svg",
    "favicon-96x96.png",
    "web-app-manifest-192x192.png",
    "web-app-manifest-512x512.png",
];

pub(crate) fn input_path(f: &str) -> PathBuf {
    Path::new(INPUT_PATH).join(f)
}

pub(crate) fn output_path(f: &str) -> PathBuf {
    Path::new(OUTPUT_PATH).join(f)
}

fn audit_enabled(content: &serde_json::Value) -> bool {
    matches!(content["website"]["audit"], serde_json::Value::Bool(true))
}

fn write_audit_scripts() -> Result<(), Box<dyn Error>> {
    let dir = output_path("audit");
    if !audit_enabled(&CONTENT) {
        if dir.exists() {
            fs::remove_dir_all(&dir)
                .map_err(|_| "failed to clear the stale audit output".to_string())?;
        }
        return Ok(());
    }
    fs::create_dir_all(&dir).map_err(|_| "failed to create the audit output".to_string())?;
    for file in ["schema-audit.js", "accessibility-tree-audit.js"] {
        fs::copy(input_path(&format!("audit/{file}")), dir.join(file))
            .map_err(|_| format!("failed to copy the audit script {file}").to_string())?;
    }
    Ok(())
}

pub(crate) static TEMPLATES: LazyLock<Tera> = LazyLock::new(|| {
    let mut t = Tera::new(TEMPLATE_PATH).unwrap_or_else(|e| {
        eprintln!("template parsing error(s): {e}");
        std::process::exit(1);
    });
    t.autoescape_on(vec![]);
    t
});

static CONTENT: LazyLock<serde_json::Value> = LazyLock::new(|| {
    let contents = fs::read_to_string(input_path(CONTENT_FILE))
        .unwrap_or_else(|e| panic!("failed to read {CONTENT_FILE}: {e}"));
    serde_json::from_str(&contents)
        .unwrap_or_else(|e| panic!("failed to parse {CONTENT_FILE}: {e}"))
});

fn print_render_error(e: &tera::Error) {
    eprintln!("template render error: {e}");
    let mut cause = e.source();
    while let Some(err) = cause {
        eprintln!("  caused by: {err}");
        cause = err.source();
    }
}

const SKIP_REGIONS: [(&str, &str); 4] = [
    ("<script", "</script>"),
    ("<style", "</style>"),
    ("<pre", "</pre>"),
    ("<textarea", "</textarea>"),
];

fn skip_region_at(b: &[u8], i: usize) -> Option<usize> {
    for (index, (open, _)) in SKIP_REGIONS.iter().enumerate() {
        let open_b = open.as_bytes();
        if b[i..].starts_with(open_b) {
            let after = b.get(i + open_b.len()).copied();
            let boundary = after.is_none_or(|c| {
                matches!(c, b' ' | b'\t' | b'\r' | b'\n' | b'>' | b'/')
            });
            if boundary {
                return Some(index);
            }
        }
    }
    None
}

fn find_sub(haystack: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    haystack[from..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + from)
}

fn conservative_minify(html: &str) -> String {
    let b = html.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'<' {
            if let Some(index) = skip_region_at(b, i) {
                let close = SKIP_REGIONS[index].1.as_bytes();
                let region_end =
                    find_sub(b, i, close).map_or(b.len(), |p| p + close.len());
                out.extend_from_slice(&b[i..region_end]);
                i = region_end;
                let mut j = i;
                let mut newline = false;
                while j < b.len() && matches!(b[j], b' ' | b'\t' | b'\r' | b'\n') {
                    newline |= b[j] == b'\n';
                    j += 1;
                }
                if newline && j < b.len() && b[j] == b'<' {
                    i = j;
                }
                continue;
            }
        }
        if b[i] == b'>' {
            let mut j = i + 1;
            let mut newline = false;
            while j < b.len() && matches!(b[j], b' ' | b'\t' | b'\r' | b'\n') {
                newline |= b[j] == b'\n';
                j += 1;
            }
            if newline && j < b.len() && b[j] == b'<' {
                out.push(b'>');
                i = j;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| html.to_owned())
}

pub(crate) fn minify_css(css: &str) -> String {
    let tighten = |c: char| matches!(c, '{' | '}' | ';' | ':' | ',' | '>');
    let chars: Vec<char> = css.chars().collect();

    let mut no_comments = String::with_capacity(css.len());
    let mut in_string = false;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '"' {
            in_string = !in_string;
            no_comments.push(c);
            i += 1;
        } else if !in_string && c == '/' && chars.get(i + 1) == Some(&'*') {
            let mut j = i + 2;
            while j + 1 < chars.len() && !(chars[j] == '*' && chars[j + 1] == '/') {
                j += 1;
            }
            i = j + 2;
        } else {
            no_comments.push(c);
            i += 1;
        }
    }

    let src: Vec<char> = no_comments.chars().collect();
    let mut out = String::with_capacity(no_comments.len());
    in_string = false;
    let mut i = 0;
    while i < src.len() {
        let c = src[i];
        if c == '"' {
            in_string = !in_string;
            out.push(c);
            i += 1;
            continue;
        }
        if in_string {
            out.push(c);
            i += 1;
            continue;
        }
        if c.is_whitespace() {
            let mut j = i;
            while j < src.len() && src[j].is_whitespace() {
                j += 1;
            }
            let next = src.get(j).copied();
            let prev = out.chars().last();
            let significant = match (prev, next) {
                (_, Some(n)) if tighten(n) => false,
                (Some(p), _) if tighten(p) => false,
                _ => true,
            };
            if significant {
                out.push(' ');
            }
            i = j;
            continue;
        }
        out.push(c);
        i += 1;
    }
    out.replace(";}", "}").trim().to_owned()
}

pub(crate) fn render_page(
    template: &str,
    ctx: &Context,
    out_file: &str,
) -> Result<(), Box<dyn Error>> {
    let rendered = TEMPLATES.render(template, ctx).map_err(|e| {
        print_render_error(&e);
        format!("failed to render template {template} for {out_file}")
    })?;

    let minified = conservative_minify(&rendered).into_bytes();
    let out = output_path(out_file);
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create directory for {out_file}: {e}"))?;
    }
    fs::write(&out, minified).map_err(|e| format!("failed to write {out_file}: {e}"))?;
    Ok(())
}

fn render_unminified(template: &str, ctx: &Context, out_file: &str) -> Result<(), Box<dyn Error>> {
    let rendered = TEMPLATES.render(template, ctx).map_err(|e| {
        print_render_error(&e);
        format!("failed to render template {template} for {out_file}")
    })?;

    fs::write(output_path(out_file), rendered)
        .map_err(|e| format!("failed to write {out_file}: {e}").into())
}

fn read_style() -> Result<String, Box<dyn Error>> {
    let css = fs::read_to_string(input_path(STYLE_FILE))
        .map_err(|e| format!("failed to read {STYLE_FILE}: {e}"))?;
    Ok(minify_css(&css))
}

fn og_images_enabled(content: &serde_json::Value) -> bool {
    content["website"]["og_images"].as_bool().unwrap_or(true)
}

fn person() -> &'static serde_json::Value {
    &CONTENT["person"]
}

fn website() -> &'static serde_json::Value {
    &CONTENT["website"]
}

fn base_context() -> Context {
    let canonical = website()["canonical"].as_str().unwrap_or("");
    let mut ctx = Context::new();
    ctx.insert("name", &person()["name"]);
    ctx.insert("title", &website()["titles"]["home"]);
    ctx.insert("description", &website()["descriptions"]["home"]);
    ctx.insert("canonical", &website()["canonical"]);
    ctx.insert("gtag", &website()["gtag"]);
    ctx.insert("lang", website()["lang"].as_str().unwrap_or(""));
    ctx.insert("og_locale", website()["og_locale"].as_str().unwrap_or(""));
    ctx.insert("labels", &website()["labels"]);
    ctx.insert("robots", ROBOTS_INDEX);
    ctx.insert("site_url", canonical);
    ctx.insert("audit", &audit_enabled(&CONTENT));
    ctx.insert("home_href", &routes::Route::home().href(routes::SLICE_PAGE));
    ctx.insert("blog_href", &routes::Route::blog().href(routes::SLICE_PAGE));
    ctx.insert(
        "tags_href",
        &routes::Route::tags_hub().href(routes::SLICE_PAGE),
    );
    ctx.insert("footer_links", &person()["links"]);
    ctx.insert("footer_home", &false);
    if og_images_enabled(&CONTENT) {
        ctx.insert("og_image", &format!("{canonical}/og.jpg"));
    }
    ctx
}

fn build_context(
    articles: &[articles::Article],
    tags: &[(&str, String, Vec<&articles::Article>)],
) -> Result<Context, Box<dyn Error>> {
    let recent_count = articles.len().min(RECENT_ARTICLES_COUNT);
    let recent_articles = &articles[..recent_count];

    let mut ctx = base_context();
    ctx.insert("note", &website()["note"]);
    ctx.insert("links", &person()["links"]);
    ctx.insert("show_feed", &!articles.is_empty());
    ctx.insert("json_ld", &schema::home_graph(&CONTENT, recent_articles).to_string());

    ctx.insert("footer_home", &true);
    let footer_tags: Vec<_> = tags
        .iter()
        .map(|(tag, slug, _)| {
            serde_json::json!({
                "name": tag_display(tag),
                "slug": slug,
                "href": routes::Route::new(&format!("/blog/tags/{slug}/")).href(routes::SLICE_PAGE),
            })
        })
        .collect();
    ctx.insert("footer_tags", &footer_tags);

    let recent: Vec<_> = recent_articles
        .iter()
        .map(|p| {
            serde_json::json!({
                "slug": p.slug,
                "title": p.frontmatter.title,
                "url": p.url(),
                "href": routes::Route::new(&p.url()).href(routes::SLICE_PAGE),
                "date": p.date_display(),
                "date_iso": p.date_iso(),
            })
        })
        .collect();
    ctx.insert("recent_articles", &recent);

    ctx.insert("style", &read_style()?);

    Ok(ctx)
}

fn build_not_found_context() -> Result<Context, Box<dyn Error>> {
    let mut ctx = base_context();
    ctx.insert("robots", ROBOTS_NOINDEX);
    ctx.insert("style", &read_style()?);
    Ok(ctx)
}

fn build_article_context(article: &articles::Article) -> Result<Context, Box<dyn Error>> {
    let canonical = website()["canonical"].as_str().unwrap_or("");

    let mut ctx = base_context();
    ctx.insert("lang", article.lang());
    ctx.insert(
        "title",
        &fill_template(
            website()["titles"]["article"].as_str().unwrap_or("{title}"),
            &[("title", &article.frontmatter.title)],
        ),
    );
    ctx.insert(
        "description",
        &fill_template(
            website()["descriptions"]["article"].as_str().unwrap_or("{description}"),
            &[("description", &article.frontmatter.description)],
        ),
    );
    ctx.insert("canonical", &format!("{canonical}{}", article.url()));
    ctx.insert("show_feed", &true);
    ctx.insert("json_ld", &schema::article_graph(&CONTENT, article).to_string());

    ctx.insert("article_title", &article.frontmatter.title);
    ctx.insert("article_date_display", &article.date_display());
    ctx.insert("article_date_iso", &article.date_iso());
    ctx.insert("article_reading_time", &article.reading_time);
    ctx.insert("article_tags", &tag_links(article.tags()));
    ctx.insert("article_content", &article.html);
    ctx.insert("og_type", "article");
    if og_images_enabled(&CONTENT) {
        ctx.insert("og_image", &format!("{canonical}{}og.jpg", article.url()));
    }
    ctx.insert("article_published", &article.date_atom());
    ctx.insert("article_publisher", canonical);
    ctx.insert(
        "markdown_source",
        &format!("{canonical}/blog/{}.md", article.slug),
    );
    let article_tags_readable: Vec<String> = article.tags().iter().map(|t| tag_display(t)).collect();
    ctx.insert("article_tags_readable", &article_tags_readable);

    let toc: Vec<_> = article
        .toc
        .iter()
        .map(|t| serde_json::json!({ "level": t.level, "text": t.text, "id": t.id }))
        .collect();
    ctx.insert("article_toc", &toc);

    ctx.insert("style", &read_style()?);

    Ok(ctx)
}

fn tag_display(tag: &str) -> String {
    website()["tags"][tag]
        .as_str()
        .unwrap_or(tag)
        .to_owned()
}

fn tag_links(tags: &[String]) -> Vec<serde_json::Value> {
    tags.iter()
        .map(|tag| {
            let slug = articles::tag_slug(tag);
            serde_json::json!({
                "name": tag,
                "display": tag_display(tag),
                "slug": slug,
                "href": routes::Route::new(&format!("/blog/tags/{slug}/")).href(routes::SLICE_PAGE),
            })
        })
        .collect()
}

fn card_values(tagged: &[&articles::Article]) -> Vec<serde_json::Value> {
    tagged
        .iter()
        .map(|p| {
            serde_json::json!({
                "slug": p.slug,
                "title": p.frontmatter.title,
                "description": p.frontmatter.description,
                "url": p.url(),
                "href": routes::Route::new(&p.url()).href(routes::SLICE_PAGE),
                "date_iso": p.date_iso(),
                "date_display": p.date_display(),
                "reading_time": p.reading_time,
                "tags": tag_links(p.tags()),
            })
        })
        .collect()
}

fn articles_count_label(count: usize) -> String {
    format!("{} {}", count, if count == 1 { "article" } else { "articles" })
}

fn description_value(keys: &[&str]) -> Result<String, Box<dyn Error>> {
    let mut node: &serde_json::Value = &website()["descriptions"];
    for key in keys {
        node = &node[key];
    }
    node.as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("descriptions.{} is missing in content.json", keys.join(".")).into())
}

fn titles_value(keys: &[&str]) -> Result<String, Box<dyn Error>> {
    let mut node: &serde_json::Value = &website()["titles"];
    for key in keys {
        node = &node[key];
    }
    node.as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("titles.{} is missing in content.json", keys.join(".")).into())
}

fn join_and(parts: &[&str]) -> String {
    match parts {
        [only] => (*only).to_string(),
        [init @ .., last] => format!("{} and {last}", init.join(", ")),
        [] => String::new(),
    }
}

fn tag_list_summary(names: &[&str], total: usize) -> String {
    let shown: Vec<&str> = names.iter().take(3).copied().collect();
    let (last, init) = shown.split_last().expect("at least one tag");
    let mut parts = init.to_vec();
    if total > 3 {
        parts.push("more");
    } else {
        parts.push(last);
    }
    join_and(&parts)
}

fn fill_template(template: &str, values: &[(&str, &str)]) -> String {
    let mut out = template.to_owned();
    for (key, value) in values {
        out = out.replace(&format!("{{{key}}}"), value);
    }
    out
}

fn tag_description(tag: &str, display: &str) -> Result<String, Box<dyn Error>> {
    match description_value(&["tags", tag]) {
        Ok(exact) => Ok(exact),
        Err(_) => Ok(fill_template(&description_value(&["tags", "any"])?, &[("tag", display)])),
    }
}

fn tag_title(tag: &str, display: &str) -> Result<String, Box<dyn Error>> {
    match titles_value(&["tags", tag]) {
        Ok(exact) => Ok(exact),
        Err(_) => Ok(fill_template(&titles_value(&["tags", "any"])?, &[("tag", display)])),
    }
}

struct LlmsLink {
    label: String,
    url: String,
    description: String,
}

struct LlmsPage {
    title: String,
    url: String,
    description: String,
    body: String,
}

fn llms_label<'a>(content: &'a serde_json::Value, key: &str) -> &'a str {
    content["website"]["labels"][key].as_str().unwrap_or("")
}

fn llms_link_line(link: &LlmsLink) -> String {
    if link.description.is_empty() {
        format!("- [{}]({})", link.label, link.url)
    } else {
        format!("- [{}]({}): {}", link.label, link.url, link.description)
    }
}

fn llms_index(content: &serde_json::Value, articles: &[LlmsLink], tags: &[LlmsLink]) -> String {
    let canonical = content["website"]["canonical"].as_str().unwrap_or("");
    let mut lines: Vec<String> = vec![
        format!("# {}", content["person"]["name"].as_str().unwrap_or("")),
        String::new(),
        format!("> {}", content["website"]["note"].as_str().unwrap_or("")),
        String::new(),
        format!("Blog: {canonical}/blog/"),
        String::new(),
        "Markdown source for every article is available by appending `.md` to the article URL.".to_owned(),
        String::new(),
        format!("## {}", llms_label(content, "blog")),
        String::new(),
    ];
    lines.extend(articles.iter().map(llms_link_line));
    lines.push(String::new());
    lines.push(format!("## {}", llms_label(content, "tags")));
    lines.push(String::new());
    lines.extend(tags.iter().map(llms_link_line));
    lines.push(String::new());
    lines.join("\n")
}

fn llms_full(content: &serde_json::Value, pages: &[LlmsPage]) -> String {
    let canonical = content["website"]["canonical"].as_str().unwrap_or("");
    let mut out = format!(
        "# {}\n\n> {}\n\nSite: {canonical}\n\n---\n\n",
        content["website"]["titles"]["home"].as_str().unwrap_or(""),
        content["website"]["note"].as_str().unwrap_or(""),
    );
    for page in pages {
        out.push_str(&format!(
            "# {}\n\nURL: {}\nSection: {}\nDescription: {}\n\n{}\n\n---\n\n",
            page.title,
            page.url,
            llms_label(content, "blog"),
            page.description,
            page.body,
        ));
    }
    out
}

fn write_llms_files(
    articles: &[articles::Article],
    tags: &[(&str, String, Vec<&articles::Article>)],
) -> Result<(), Box<dyn Error>> {
    let canonical = website()["canonical"].as_str().unwrap_or("").trim_end_matches('/');

    let article_links: Vec<LlmsLink> = articles
        .iter()
        .map(|p| LlmsLink {
            label: p.frontmatter.title.clone(),
            url: format!("{canonical}{}", p.url()),
            description: p.frontmatter.description.clone(),
        })
        .collect();

    let tag_links: Vec<LlmsLink> = tags
        .iter()
        .map(|(tag, slug, _)| {
            let display = tag_display(tag);
            let description = tag_description(tag, &display).unwrap_or_default();
            LlmsLink {
                label: display,
                url: format!("{canonical}/blog/tags/{slug}/"),
                description,
            }
        })
        .collect();

    let pages: Vec<LlmsPage> = articles
        .iter()
        .map(|p| LlmsPage {
            title: p.frontmatter.title.clone(),
            url: format!("{canonical}{}#page", p.url()),
            description: p.frontmatter.description.clone(),
            body: p.body.clone(),
        })
        .collect();

    fs::write(output_path("llms.txt"), llms_index(&CONTENT, &article_links, &tag_links))
        .map_err(|_| "failed to write llms.txt".to_string())?;
    fs::write(output_path("llms-full.txt"), llms_full(&CONTENT, &pages))
        .map_err(|_| "failed to write llms-full.txt".to_string())?;

    for article in articles {
        fs::write(output_path(&format!("blog/{}.md", article.slug)), &article.body).map_err(|_| {
            format!("failed to write the markdown source of article {}", article.slug).to_string()
        })?;
    }
    Ok(())
}

fn build_blog_index_context(articles: &[articles::Article]) -> Result<Context, Box<dyn Error>> {
    let canonical = website()["canonical"].as_str().unwrap_or("");
    let description = description_value(&["blog"])?;

    let mut ctx = base_context();
    ctx.insert("title", &titles_value(&["blog"])?);
    ctx.insert("description", &description);
    ctx.insert("canonical", &format!("{canonical}/blog/"));
    ctx.insert("show_feed", &true);
    ctx.insert(
        "json_ld",
        &schema::blog_index_graph(&CONTENT, articles, &description).to_string(),
    );
    ctx.insert("blog_count_label", &articles_count_label(articles.len()));

    let cards: Vec<&articles::Article> = articles.iter().collect();
    ctx.insert("cards", &card_values(&cards));
    ctx.insert("style", &read_style()?);

    Ok(ctx)
}

fn build_tag_context(
    tag: &str,
    slug: &str,
    tagged: &[&articles::Article],
) -> Result<Context, Box<dyn Error>> {
    let canonical = website()["canonical"].as_str().unwrap_or("");

    let mut ctx = base_context();
    let display = tag_display(tag);
    ctx.insert("title", &tag_title(tag, &display)?);
    let description = tag_description(tag, &display)?;
    ctx.insert("description", &description);
    ctx.insert("canonical", &format!("{canonical}/blog/tags/{slug}/"));
    ctx.insert(
        "json_ld",
        &schema::tag_graph(&CONTENT, tag, slug, tagged, &description).to_string(),
    );
    ctx.insert("tag_name", &display);
    ctx.insert("tag_count_label", &articles_count_label(tagged.len()));
    ctx.insert("cards", &card_values(tagged));
    ctx.insert("style", &read_style()?);

    Ok(ctx)
}

fn build_tags_index_context(
    tags: &[(&str, String, Vec<&articles::Article>)],
) -> Result<Context, Box<dyn Error>> {
    let canonical = website()["canonical"].as_str().unwrap_or("");

    let tag_values: Vec<_> = tags
        .iter()
        .map(|(tag, slug, tagged)| {
            serde_json::json!({
                "name": tag_display(tag),
                "slug": slug,
                "count": tagged.len(),
                "href": routes::Route::new(&format!("/blog/tags/{slug}/")).href(routes::SLICE_PAGE),
            })
        })
        .collect();

    let mut ctx = base_context();
    ctx.insert("title", &titles_value(&["tags", "all"])?);
    let display_names: Vec<String> = tags.iter().map(|(tag, _, _)| tag_display(tag)).collect();
    let names: Vec<&str> = display_names.iter().map(String::as_str).collect();
    let tag_list = tag_list_summary(&names, tags.len());
    let description = fill_template(&description_value(&["tags", "all"])?, &[("tags", &tag_list)]);
    ctx.insert("description", &description);
    ctx.insert("canonical", &format!("{canonical}/blog/tags/"));
    ctx.insert(
        "json_ld",
        &schema::tags_index_graph(&CONTENT, tags, &description).to_string(),
    );
    ctx.insert("tags", &tag_values);
    ctx.insert("style", &read_style()?);

    Ok(ctx)
}

fn build_atom_context(articles: &[articles::Article]) -> Result<Context, Box<dyn Error>> {
    let canonical = website()["canonical"].as_str().unwrap_or("");

    let entries: Vec<_> = articles
        .iter()
        .map(|p| {
            serde_json::json!({
                "title": markdown::escape_html(&p.frontmatter.title),
                "url": format!("{canonical}{}", p.url()),
                "id": format!("{canonical}{}", p.url()),
                "published": p.date_atom(),
                "updated": p.modified_atom,
                "summary": markdown::escape_html(&p.frontmatter.description),
            })
        })
        .collect();

    let mut ctx = Context::new();
    ctx.insert("name", &person()["name"]);
    ctx.insert(
        "description",
        &markdown::escape_html(website()["note"].as_str().unwrap_or("")),
    );
    ctx.insert("canonical", canonical);
    ctx.insert(
        "feed_updated",
        &articles
            .first()
            .map(|p| p.modified_atom.clone())
            .unwrap_or_default(),
    );
    ctx.insert("entries", &entries);

    Ok(ctx)
}

fn copy_static_files() -> Result<(), Box<dyn Error>> {
    for f in COPY_FILES {
        fs::copy(input_path(f), output_path(f))
            .map_err(|e| format!("failed to copy {f}: {e}"))?;
    }
    Ok(())
}

fn article_output_file(article: &articles::Article) -> String {
    format!("blog/{}/index.html", article.slug)
}

fn write_diagrams(article: &articles::Article) -> Result<(), Box<dyn Error>> {
    if article.diagrams.is_empty() {
        return Ok(());
    }
    let dir = output_path(&format!("blog/{}/diagrams", article.slug));
    fs::create_dir_all(&dir)
        .map_err(|e| format!("failed to create diagrams dir for {}: {e}", article.slug))?;
    for diagram in &article.diagrams {
        fs::write(dir.join(&diagram.file_name), &diagram.svg)
            .map_err(|e| format!("failed to write {}: {e}", diagram.file_name))?;
    }
    Ok(())
}

fn audit_output(page_path: &str, out_file: &str) -> Result<(), Box<dyn Error>> {
    let html = fs::read_to_string(output_path(out_file))
        .map_err(|e| format!("failed to read back {out_file}: {e}"))?;
    let canonical = website()["canonical"].as_str().unwrap_or("");

    schema::audit_page(canonical, page_path, &html)
        .map_err(|issues| format!("\n{}", schema::audit_report(out_file, &issues)))?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let articles = articles::collect_articles(&input_path(ARTICLES_INPUT_DIR))?;
    let tags = articles::collect_tags(&articles);

    if output_path("blog").exists() {
        fs::remove_dir_all(output_path("blog"))
            .map_err(|_| "failed to clear the stale blog output".to_string())?;
    }

    let ctx = build_context(&articles, &tags)?;
    let not_found_ctx = build_not_found_context()?;

    render_page(INDEX_TEMPLATE_FILE, &ctx, INDEX_OUTPUT_FILE)?;
    render_page(NOT_FOUND_TEMPLATE_FILE, &not_found_ctx, NOT_FOUND_OUTPUT_FILE)?;

    for article in &articles {
        let article_ctx = build_article_context(article)?;
        render_page(ARTICLE_TEMPLATE_FILE, &article_ctx, &article_output_file(article))?;
        write_diagrams(article)?;
    }

    if !articles.is_empty() {
        let blog_ctx = build_blog_index_context(&articles)?;
        render_page(BLOG_TEMPLATE_FILE, &blog_ctx, BLOG_INDEX_OUTPUT_FILE)?;
    }

    for (tag, slug, tagged) in &tags {
        let tag_ctx = build_tag_context(tag, slug, tagged)?;
        render_page(TAG_TEMPLATE_FILE, &tag_ctx, &tag_output_file(slug))?;
    }

    if !tags.is_empty() {
        let tags_ctx = build_tags_index_context(&tags)?;
        render_page(TAGS_TEMPLATE_FILE, &tags_ctx, TAGS_INDEX_OUTPUT_FILE)?;
    }

    if !articles.is_empty() {
        let atom_ctx = build_atom_context(&articles)?;
        render_unminified(ATOM_TEMPLATE_FILE, &atom_ctx, ATOM_OUTPUT_FILE)?;
    }

    write_og_images(&articles)?;
    write_sitemaps(&articles, &tags)?;
    write_llms_files(&articles, &tags)?;
    copy_static_files()?;
    write_audit_scripts()?;

    audit_output("/", INDEX_OUTPUT_FILE)?;
    for article in &articles {
        audit_output(&article.url(), &article_output_file(article))?;
    }
    if !articles.is_empty() {
        audit_output("/blog/", BLOG_INDEX_OUTPUT_FILE)?;
    }
    for (_, slug, _) in &tags {
        audit_output(&format!("/blog/tags/{slug}/"), &tag_output_file(slug))?;
    }
    if !tags.is_empty() {
        audit_output("/blog/tags/", TAGS_INDEX_OUTPUT_FILE)?;
    }

    Ok(())
}

fn tag_output_file(slug: &str) -> String {
    format!("blog/tags/{slug}/index.html")
}

const TAGS_INDEX_OUTPUT_FILE: &str = "blog/tags/index.html";

fn write_og_images(articles: &[articles::Article]) -> Result<(), Box<dyn Error>> {
    if !og_images_enabled(&CONTENT) {
        let stale = output_path("og.jpg");
        if stale.exists() {
            fs::remove_file(&stale)
                .map_err(|e| format!("failed to remove the stale root og image: {e}"))?;
        }
        return Ok(());
    }

    let site_line = website()["canonical"]
        .as_str()
        .unwrap_or("")
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let bg = input_path(OG_BACKGROUND_FILE);

    for article in articles {
        let out = output_path(&format!("blog/{}/og.jpg", article.slug));
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create og dir for {}: {e}", article.slug))?;
        }
        og::render(
            &bg,
            &article.frontmatter.title,
            &article.frontmatter.description,
            site_line,
            &out,
        )?;
    }

    og::render(
        &bg,
        website()["titles"]["home"].as_str().unwrap_or(""),
        website()["descriptions"]["home"].as_str().unwrap_or(""),
        site_line,
        &output_path("og.jpg"),
    )
}

fn sitemap_url(loc: &str, lastmod: Option<&str>) -> String {
    let lastmod = lastmod
        .map(|d| format!("\n    <lastmod>{d}</lastmod>"))
        .unwrap_or_default();
    format!(
        "  <url>\n    <loc>{}</loc>{}\n  </url>",
        markdown::escape_html(loc),
        lastmod
    )
}

fn sitemap_document(urls: &[String]) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n{}\n</urlset>\n",
        urls.join("\n")
    )
}

fn sitemap_index_entry(loc: &str) -> String {
    format!(
        "  <sitemap>\n    <loc>{}</loc>\n  </sitemap>",
        markdown::escape_html(loc)
    )
}

fn write_sitemaps(
    articles: &[articles::Article],
    tags: &[(&str, String, Vec<&articles::Article>)],
) -> Result<(), Box<dyn Error>> {
    let canonical = website()["canonical"].as_str().unwrap_or("");
    let newest = articles
        .first()
        .map(|p| p.modified_atom.clone())
        .unwrap_or_default();
    let newest = if newest.is_empty() { None } else { Some(&newest[..]) };

    let mut urls = vec![
        sitemap_url(canonical, newest),
        sitemap_url(&format!("{canonical}/blog/"), newest),
        sitemap_url(&format!("{canonical}/blog/tags/"), newest),
    ];
    for article in articles {
        urls.push(sitemap_url(
            &format!("{canonical}{}", article.url()),
            Some(&article.modified_atom),
        ));
    }
    for (_, slug, tagged) in tags {
        let tag_newest = tagged
            .first()
            .map(|p| p.modified_atom.clone())
            .unwrap_or_default();
        urls.push(sitemap_url(
            &format!("{canonical}/blog/tags/{slug}/"),
            if tag_newest.is_empty() { None } else { Some(&tag_newest) },
        ));
    }

    let index = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<sitemapindex xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">
{}
</sitemapindex>
",
        sitemap_index_entry(&format!("{canonical}/sitemap-0.xml"))
    );

    fs::write(output_path("sitemap-0.xml"), sitemap_document(&urls))
        .map_err(|e| format!("failed to write sitemap-0.xml: {e}"))?;
    fs::write(output_path("sitemap-index.xml"), index)
        .map_err(|e| format!("failed to write sitemap-index.xml: {e}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_list_summary_joins_up_to_three_names() {
        assert_eq!(tag_list_summary(&["ai"], 1), "ai");
        assert_eq!(tag_list_summary(&["ai", "graphtheory"], 2), "ai and graphtheory");
        assert_eq!(
            tag_list_summary(&["ai", "graphtheory", "programming"], 3),
            "ai, graphtheory and programming"
        );
    }

    #[test]
    fn tag_list_summary_appends_more_only_past_three() {
        assert_eq!(
            tag_list_summary(&["a", "b", "c", "d"], 4),
            "a, b and more"
        );
    }

    #[test]
    fn fill_template_replaces_placeholders() {
        assert_eq!(
            fill_template("Articles related to {tag}, newest first.", &[("tag", "ai")]),
            "Articles related to ai, newest first."
        );
        assert_eq!(
            fill_template("grouped by tag: {tags}.", &[("tags", "a, b and c")]),
            "grouped by tag: a, b and c."
        );
    }

    #[test]
    fn tag_title_prefers_exact_entries() {
        assert_eq!(
            tag_title("graphtheory", "Graph theory").unwrap(),
            "Articles related to graph theory"
        );
        assert_eq!(
            tag_title("nosuchtag", "Made up").unwrap(),
            "Articles related to Made up"
        );
    }

    #[test]
    fn og_images_enabled_defaults_to_true() {
        assert!(og_images_enabled(&serde_json::json!({})));
        assert!(og_images_enabled(&serde_json::json!({ "website": { "og_images": "yes" } })));
        assert!(og_images_enabled(&serde_json::json!({ "website": { "og_images": true } })));
        assert!(!og_images_enabled(&serde_json::json!({ "website": { "og_images": false } })));
    }

    fn base_test_context() -> Context {
        let mut ctx = Context::new();
        ctx.insert("title", "Title");
        ctx.insert("name", "Name");
        ctx.insert("description", "Description");
        ctx.insert("canonical", "https://example.com");
        ctx.insert("gtag", "G-1");
        ctx.insert("lang", "en");
        ctx.insert("og_locale", "en_US");
        ctx.insert("style", "");
        ctx.insert("home_href", "/#page");
        ctx.insert("blog_href", "/blog/#page");
        ctx.insert("tags_href", "/blog/tags/#page");
        ctx
    }

    #[test]
    fn base_template_gates_image_meta_on_og_image() {
        fn render_base(og_image: Option<&str>) -> String {
            let mut ctx = base_test_context();
            if let Some(og) = og_image {
                ctx.insert("og_image", og);
            }
            TEMPLATES.render("base.hbs", &ctx).unwrap()
        }

        let with = render_base(Some("https://example.com/og.jpg"));
        assert!(with.contains(r#"content="summary_large_image""#));
        assert!(with.contains("og:image"));

        let without = render_base(None);
        assert!(without.contains(r#"content="summary""#));
        assert!(!without.contains("summary_large_image"));
        assert!(!without.contains("og:image"));
        assert!(!without.contains("twitter:image"));
    }

    #[test]
    fn base_template_gates_audit_markup_on_the_flag() {
        let render = |audit: bool| {
            let mut ctx = base_test_context();
            ctx.insert("site_url", "https://example.com");
            ctx.insert("audit", &audit);
            TEMPLATES.render("base.hbs", &ctx).unwrap()
        };

        let with = render(true);
        assert!(with.contains("window.__SITE_AUDIT__=true"));
        assert!(with.contains(r#"window.__SITE_AUDIT_URL__="https://example.com""#));
        assert!(with.contains(r#"<script src="/audit/schema-audit.js">"#));
        assert!(with.contains(r#"<script src="/audit/accessibility-tree-audit.js">"#));

        let without = render(false);
        assert!(!without.contains("__SITE_AUDIT__"));
        assert!(!without.contains("/audit/"));
    }

    #[test]
    fn audit_flag_defaults_to_false() {
        assert!(!audit_enabled(&serde_json::json!({})));
        assert!(!audit_enabled(&serde_json::json!({ "website": { "audit": "yes" } })));
        assert!(audit_enabled(&serde_json::json!({ "website": { "audit": true } })));
        assert!(!audit_enabled(&serde_json::json!({ "website": { "audit": false } })));
    }

    #[test]
    fn conservative_minify_collapses_only_inter_tag_newlines() {
        let src = "<html lang=\"en\" id=\"web\">\n  <head>\n    <title>A</title>\n  </head>\n  <body>\n    <p>By <a href=\"/x\">link</a> tail</p>\n  </body>\n</html>\n";
        let out = conservative_minify(src);
        assert_eq!(
            out,
            "<html lang=\"en\" id=\"web\"><head><title>A</title></head><body><p>By <a href=\"/x\">link</a> tail</p></body></html>\n"
        );
        assert!(out.contains("lang=\"en\""));
    }

    #[test]
    fn conservative_minify_keeps_skip_regions_verbatim() {
        let pre = "<pre><code>a\n  b   c\n</code></pre>";
        let script = "<script>\n  var x = 1 > 0 ? a : b;\n</script>";
        let style = "<style>\n  p { color: red; }\n</style>";
        let src = format!("<main>\n  {pre}\n  {script}\n  {style}\n</main>\n");
        let out = conservative_minify(&src);
        assert!(out.contains(pre));
        assert!(out.contains(script));
        assert!(out.contains(style));
        assert!(out.starts_with("<main>"));
        assert!(out.ends_with("</style></main>\n"));
    }

    #[test]
    fn minify_css_strips_comments_and_tightens_punctuation() {
        let src = "/* header */\np {\n  color: red;\n  font-family: \"SF Mono\", Menlo;\n}\n";
        let out = minify_css(src);
        assert_eq!(out, "p{color:red;font-family:\"SF Mono\",Menlo}");
    }

    #[test]
    fn llms_generators_follow_the_reference_format() {
        let content = serde_json::json!({
            "person": { "name": "Someone" },
            "website": {
                "canonical": "https://example.com",
                "note": "Desc",
                "titles": { "home": "Someone" },
                "labels": { "blog": "Journal", "tags": "Topics" }
            }
        });
        let articles = vec![LlmsLink {
            label: "Article one".to_owned(),
            url: "https://example.com/blog/one/".to_owned(),
            description: "First".to_owned(),
        }];
        let tags = vec![LlmsLink {
            label: "Graph theory".to_owned(),
            url: "https://example.com/blog/tags/graphtheory/".to_owned(),
            description: String::new(),
        }];

        let index = llms_index(&content, &articles, &tags);
        assert!(index.contains("# Someone\n\n> Desc\n\nBlog: https://example.com/blog/"));
        assert!(index.contains("Markdown source for every article is available by appending `.md`"));
        assert!(index.contains("## Journal"));
        assert!(index.contains("- [Article one](https://example.com/blog/one/): First"));
        assert!(index.contains("## Topics"));
        assert!(index.contains("- [Graph theory](https://example.com/blog/tags/graphtheory/)"));

        let pages = vec![LlmsPage {
            title: "Article one".to_owned(),
            url: "https://example.com/blog/one/#page".to_owned(),
            description: "First".to_owned(),
            body: "Body text.".to_owned(),
        }];
        let full = llms_full(&content, &pages);
        assert!(full.starts_with("# Someone\n\n> Desc\n\nSite: https://example.com\n\n---\n\n"));
        assert!(full.contains(
            "# Article one\n\nURL: https://example.com/blog/one/#page\nSection: Journal\nDescription: First\n\nBody text.\n\n---\n\n"
        ));
    }

    #[test]
    fn base_template_gates_markdown_source_link_on_articles() {
        let render = |source: Option<&str>| {
            let mut ctx = base_test_context();
            ctx.insert("canonical", "https://example.com/blog/one/");
            if let Some(url) = source {
                ctx.insert("markdown_source", url);
                ctx.insert("article_title", "Article one");
            }
            TEMPLATES.render("base.hbs", &ctx).unwrap()
        };

        let with = render(Some("https://example.com/blog/one.md"));
        assert!(with.contains(
            r#"<link rel="alternate" type="text/markdown" href="https://example.com/blog/one.md" title="Article one - Markdown source">"#
        ));
        assert!(!with.contains("/>"));

        let without = render(None);
        assert!(!without.contains("text/markdown"));
    }

    #[test]
    fn base_template_gates_publisher_meta_on_article_context() {
        let render = |publisher: Option<&str>| {
            let mut ctx = base_test_context();
            if let Some(url) = publisher {
                ctx.insert("article_publisher", url);
            }
            TEMPLATES.render("base.hbs", &ctx).unwrap()
        };

        let with = render(Some("https://example.com"));
        assert!(with.contains(r#"<meta property="article:publisher" content="https://example.com">"#));

        let without = render(None);
        assert!(!without.contains("article:publisher"));
    }

    #[test]
    fn base_template_renders_footer_variants() {
        fn render_footer(home: bool) -> String {
            let mut ctx = base_test_context();
            ctx.insert("footer_home", &home);
            ctx.insert(
                "footer_tags",
                &[serde_json::json!({ "name": "Graph theory", "slug": "graphtheory", "href": "/blog/tags/graphtheory/#page" })],
            );
            ctx.insert(
                "footer_links",
                &[serde_json::json!({ "title": "Github", "href": "https://github.com/example" })],
            );
            TEMPLATES.render("base.hbs", &ctx).unwrap()
        }

        let home = render_footer(true);
        assert!(home.contains(r#"<nav aria-label="Footer menu">"#));
        assert!(home.contains(r#"href="/blog/tags/graphtheory/#page""#));
        assert!(home.contains("Graph theory"));
        assert!(!home.contains("Back to home"));
        assert!(!home.contains("github.com"));
        assert!(home.contains(r#"href="/llms.txt""#));
        assert!(home.contains(r#"href="/llms-full.txt""#));
        assert!(!home.contains("footer-inner"));
        assert!(!home.contains("footer-home"));

        let other = render_footer(false);
        assert!(other.contains(r#"<nav aria-label="Footer menu">"#));
        assert!(other.contains(r#"href="https://github.com/example""#));
        assert!(other.contains(r#"href="/#page">Back to home"#));
        assert!(other.contains(r#"href="/llms.txt""#));
        assert!(other.contains(r#"href="/llms-full.txt""#));
        assert!(!other.contains("/blog/tags/"));
        assert!(!other.contains("footer-inner"));
        assert!(!other.contains("footer-home"));
    }

    #[test]
    fn home_template_renders_labels_from_context() {
        let labels = serde_json::json!({
            "home": "Start", "blog": "Journal", "tags": "Topics", "links": "Connections",
            "recent": "Fresh writing", "articles": "Entries", "toc": "On this page",
            "toc_list": "Table of contents"
        });
        let mut ctx = base_test_context();
        ctx.insert("labels", &labels);
        ctx.insert("note", "Note");
        ctx.insert("links", &[] as &[serde_json::Value]);
        ctx.insert(
            "recent_articles",
            &[serde_json::json!({ "slug": "a", "title": "T", "url": "/blog/a/", "href": "/blog/a/#page", "date": "Sep 5, 2026", "date_iso": "2026-09-05" })],
        );
        let html = TEMPLATES.render("index.hbs", &ctx).unwrap();

        assert!(html.contains(r#"<section class="content" id="person" tabindex="-1" aria-labelledby="person-title">"#));
        assert!(html.contains(r#"<h1 id="person-title">"#));
        assert!(html.contains(r#"<h2 id="links-title">Connections</h2>"#));
        assert!(html.contains(r#"<ul id="links-list" aria-labelledby="links-title">"#));
        assert!(html.contains(r#"<h2 id="blog-title">Fresh writing</h2>"#));
        assert!(html.contains(r#"<ul id="blog-articles" aria-labelledby="blog-title">"#));
        assert!(html.contains(r#"<a href="/blog/a/#page">T</a>"#));
        assert!(html.contains(r#"<a href="/blog/#page">All articles"#));
        assert!(!html.contains("Recent articles"));
    }

    #[test]
    fn article_template_toc_and_chips_carry_accessible_names() {
        let labels = serde_json::json!({
            "home": "Start", "blog": "Journal", "tags": "Topics", "links": "Connections",
            "recent": "Fresh writing", "articles": "Entries", "toc": "On this page",
            "toc_list": "Table of contents"
        });
        let mut ctx = base_test_context();
        ctx.insert("labels", &labels);
        ctx.insert("article_title", "Title");
        ctx.insert("article_date_display", "Sep 5, 2026");
        ctx.insert("article_date_iso", "2026-09-05");
        ctx.insert("article_reading_time", &1);
        ctx.insert(
            "article_tags",
            &[serde_json::json!({ "name": "graphtheory", "display": "Graph theory", "slug": "graphtheory", "href": "/blog/tags/graphtheory/#page" })],
        );
        ctx.insert("article_content", "Body");
        ctx.insert("article_toc", &[serde_json::json!({ "level": 2, "text": "Intro", "id": "intro" })]);
        let html = TEMPLATES.render("article.hbs", &ctx).unwrap();

        assert!(html.contains(r#"<article class="article" id="article" tabindex="-1" aria-labelledby="article-title">"#));
        assert!(html.contains(r#"<h1 id="article-title">Title</h1>"#));
        assert!(html.contains(r#"aria-labelledby="toc-title""#));
        assert!(html.contains(r#"<h2 id="toc-title">On this page</h2>"#));
        assert!(html.contains(r#"aria-label="Table of contents""#));
        assert!(html.contains(r#"title="Graph theory""#));
        assert!(html.contains(r#"aria-label="Graph theory""#));
        assert!(html.contains(r#"href="/blog/tags/graphtheory/#page""#));
        assert!(html.contains(r#"<header class="header"><a href="/#page">Name</a></header>"#));
        assert!(html.contains("#graphtheory"));
    }

    #[test]
    fn tag_template_articles_list_is_named_by_the_tag() {
        let labels = serde_json::json!({
            "home": "Start", "blog": "Journal", "tags": "Topics", "links": "Connections",
            "recent": "Fresh writing", "articles": "Entries", "toc": "On this page",
            "toc_list": "Table of contents"
        });
        let mut ctx = base_test_context();
        ctx.insert("labels", &labels);
        ctx.insert("tag_name", "Graph theory");
        ctx.insert("tag_count_label", "1 article");
        ctx.insert("cards", &[] as &[serde_json::Value]);
        let html = TEMPLATES.render("tag.hbs", &ctx).unwrap();

        assert!(html.contains(r#"aria-labelledby="tag-title""#));
        assert!(!html.contains(r#"aria-label="Entries""#));
    }
}
