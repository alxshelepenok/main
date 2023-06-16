use std::error::Error;

use comrak::nodes::{NodeCodeBlock, NodeHtmlBlock, NodeValue};
use comrak::{format_html, parse_document, Anchorizer, Arena, Options};

use crate::mermaid;

pub struct TocItem {
    pub level: u8,
    pub text: String,
    pub id: String,
}

pub struct DiagramAsset {
    pub file_name: String,
    pub svg: String,
}

pub struct Rendered {
    pub html: String,
    pub toc: Vec<TocItem>,
    pub diagrams: Vec<DiagramAsset>,
}

pub fn options() -> Options<'static> {
    let mut options = Options::default();
    let extension = &mut options.extension;
    extension.front_matter_delimiter = Some("+++".to_owned());
    extension.table = true;
    extension.tasklist = true;
    extension.strikethrough = true;
    extension.autolink = true;
    extension.header_id_prefix = Some(String::new());
    options.render.r#unsafe = true;
    options
}

pub fn render(body: &str, diagram_base: &str) -> Result<Rendered, Box<dyn Error>> {
    let arena = Arena::new();
    let options = options();
    let root = parse_document(&arena, body, &options);

    let mut anchorizer = Anchorizer::new();
    let mut toc = Vec::new();
    let mut diagrams = Vec::new();
    let mut mermaid_blocks = Vec::new();

    for node in root.descendants() {
        match &node.data.borrow().value {
            NodeValue::Heading(nh) => {
                let text = node.collect_text();
                let id = anchorizer.anchorize(&text);
                if (2..=3).contains(&nh.level) {
                    toc.push(TocItem { level: nh.level, text, id });
                }
            }
            NodeValue::CodeBlock(cb) if is_mermaid(cb) => mermaid_blocks.push(node),
            _ => {}
        }
    }

    for (no, node) in mermaid_blocks.into_iter().enumerate() {
        let (source, title) = match &node.data.borrow().value {
            NodeValue::CodeBlock(cb) => {
                let source = cb.literal.trim_end().to_owned();
                let title = cb
                    .info
                    .split_once(char::is_whitespace)
                    .map(|(_, rest)| rest.trim().to_owned())
                    .filter(|rest| !rest.is_empty())
                    .unwrap_or_else(|| "Diagram".to_owned());
                (source, title)
            }
            _ => unreachable!("collected node is a code block"),
        };

        let file_name = format!("diagram-{}.svg", no + 1);
        let svg = mermaid::render_svg(&source)?;
        diagrams.push(DiagramAsset {
            file_name: file_name.clone(),
            svg: svg.clone(),
        });

        let figure = diagram_figure(
            &format!("{diagram_base}/{file_name}"),
            &title,
            &source,
            &svg,
            &format!("diagram-{}", no + 1),
        );
        let replacement = arena.alloc(
            NodeValue::HtmlBlock(NodeHtmlBlock { block_type: 6, literal: figure }).into(),
        );
        node.insert_before(replacement);
        node.detach();
    }

    let mut html = String::new();
    format_html(root, &options, &mut html)?;

    Ok(Rendered { html, toc, diagrams })
}

fn is_mermaid(cb: &NodeCodeBlock) -> bool {
    cb.fenced && cb.info.split_whitespace().next() == Some("mermaid")
}

fn diagram_figure(src: &str, title: &str, source: &str, svg: &str, name: &str) -> String {
    let (width, height) = svg_dimensions(svg);
    let size = |attr: &str, value: Option<u32>| match value {
        Some(v) => format!(" {attr}=\"{v}\""),
        None => String::new(),
    };

    format!(
        "<figure class=\"diagram tabs\">\n\
         <details name=\"{name}\" open>\n\
         <summary>Diagram</summary>\n\
         <div class=\"diagram-content\"><img src=\"{src}\" alt=\"{title_attr}\"{width_attr}{height_attr} loading=\"lazy\" decoding=\"async\"></div>\n\
         </details>\n\
         <details name=\"{name}\">\n\
         <summary>Code</summary>\n\
         <div class=\"diagram-content\"><pre><code class=\"language-mermaid\">{escaped}</code></pre></div>\n\
         </details>\n\
         </figure>\n",
        name = name,
        src = src,
        title_attr = escape_html(title),
        width_attr = size("width", width),
        height_attr = size("height", height),
        escaped = escape_html(source),
    )
}

fn svg_dimensions(svg: &str) -> (Option<u32>, Option<u32>) {
    let root = &svg[..svg.find('>').unwrap_or(svg.len())];
    let attr = |name: &str| -> Option<String> {
        let marker = format!("{name}=");
        let pos = root.find(&marker)?;
        let rest = &root[pos + marker.len()..];
        let value = if let Some(stripped) = rest.strip_prefix('"') {
            stripped.split('"').next()
        } else {
            rest.split(|c: char| c.is_whitespace() || c == '>').next()
        };
        value.map(str::to_owned)
    };
    let parse_px = |value: Option<String>| -> Option<f64> {
        value?
            .trim_end_matches("px")
            .parse::<f64>()
            .ok()
            .filter(|v| *v > 0.0)
    };

    let mut width = parse_px(attr("width"));
    let mut height = parse_px(attr("height"));
    if width.is_none() || height.is_none() {
        if let Some(view_box) = attr("viewBox") {
            let parts: Vec<&str> = view_box.split_whitespace().collect();
            if parts.len() == 4 {
                width = width.or_else(|| parts[2].parse::<f64>().ok());
                height = height.or_else(|| parts[3].parse::<f64>().ok());
            }
        }
    }

    (width.map(|v| v.round() as u32), height.map(|v| v.round() as u32))
}

pub fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toc_ids_match_heading_ids_with_dedup() {
        let rendered = render("## Alpha\n### Alpha\n", "/diagrams").unwrap();
        assert_eq!(rendered.toc.len(), 2);
        assert_eq!(rendered.toc[0].id, "alpha");
        assert_eq!(rendered.toc[1].id, "alpha-1");
        assert!(rendered.html.contains("<h2 id=\"alpha\""));
        assert!(rendered.html.contains("<h3 id=\"alpha-1\""));
    }

    #[test]
    fn headings_outside_toc_keep_dedup_counters_in_sync() {
        let rendered = render("# Same\n## Same\n", "/diagrams").unwrap();
        assert_eq!(rendered.toc.len(), 1);
        assert_eq!(rendered.toc[0].id, "same-1");
        assert!(rendered.html.contains("<h2 id=\"same-1\""));
    }

    #[test]
    fn mermaid_block_becomes_img_figure() {
        let md = "```mermaid\nflowchart TD\nA[Start] --> B[Done]\n```";
        let rendered = render(md, "/diagrams").unwrap();
        assert!(rendered.html.contains("<figure class=\"diagram tabs\">"));
        assert!(rendered.html.contains("<details name=\"diagram-1\" open>"));
        assert_eq!(
            rendered.html.matches("name=\"diagram-1\"").count(),
            2,
            "panels of one figure share an exclusive tab group"
        );
        assert!(
            rendered
                .html
                .contains("<img src=\"/diagrams/diagram-1.svg\" alt=\"Diagram\" width=")
        );
        assert!(rendered.html.contains(" loading=\"lazy\" decoding=\"async\""));
        assert!(rendered.html.contains("class=\"language-mermaid\""));
        assert!(rendered
            .html
            .contains("<div class=\"diagram-content\"><pre><code class=\"language-mermaid\">"));
        assert!(!rendered.html.contains("<svg"), "no svg subtree in the page dom");
        assert_eq!(rendered.html.matches("<details name=\"diagram-1\" open>").count(), 1);
        assert_eq!(rendered.diagrams.len(), 1);
        assert_eq!(rendered.diagrams[0].file_name, "diagram-1.svg");
        assert!(rendered.diagrams[0].svg.trim_start().to_lowercase().starts_with("<svg"));
        assert_eq!(rendered.html.matches("<figure").count(), 1);
    }

    #[test]
    fn multiple_figures_get_unique_tab_groups() {
        let md = "```mermaid\nflowchart TD\nA --> B\n```\n\nText\n\n```mermaid\nflowchart TD\nC --> D\n```";
        let rendered = render(md, "/diagrams").unwrap();
        assert_eq!(rendered.html.matches("name=\"diagram-1\"").count(), 2);
        assert_eq!(rendered.html.matches("name=\"diagram-2\"").count(), 2);
        assert_eq!(rendered.html.matches("<figure").count(), 2);
    }

    #[test]
    fn mermaid_info_string_title_becomes_alt() {
        let md = "```mermaid Deploy flow\nflowchart TD\nA --> B\n```";
        let rendered = render(md, "/diagrams").unwrap();
        assert!(rendered.html.contains("alt=\"Deploy flow\""));
    }

    #[test]
    fn strong_stays_semantic_and_raw_b_passes_through() {
        let rendered = render("This is **important** and this is <b>visual</b>.\n", "/diagrams")
            .unwrap();
        assert!(rendered.html.contains("<strong>important</strong>"));
        assert!(rendered.html.contains("<b>visual</b>"));
    }

    #[test]
    fn mermaid_syntax_error_fails_render() {
        let md = "```mermaid\nthis is not a diagram\n```";
        assert!(render(md, "/diagrams").is_err());
    }

    #[test]
    fn code_block_source_is_escaped() {
        let md = "```mermaid\nflowchart TD\nA[\"a < b & c\"] --> B\n```";
        let rendered = render(md, "/diagrams").unwrap();
        assert!(rendered.html.contains("&lt; b &amp; c"));
    }

    #[test]
    fn frontmatter_is_not_rendered() {
        let body = "+++\ntitle = \"x\"\n+++\n\nText";
        let rendered = render(body, "/diagrams").unwrap();
        assert!(rendered.html.contains("<p>Text</p>"));
        assert!(!rendered.html.contains("title"));
    }

    #[test]
    fn svg_dimensions_prefer_explicit_pixels() {
        let svg = r#"<svg width="640px" height="480" viewBox="0 0 320 240">"#;
        assert_eq!(svg_dimensions(svg), (Some(640), Some(480)));
    }

    #[test]
    fn svg_dimensions_fall_back_to_view_box() {
        let svg = r#"<svg width="100%" viewBox="0 0 111.875 174">"#;
        assert_eq!(svg_dimensions(svg), (Some(112), Some(174)));
    }

    #[test]
    fn svg_dimensions_without_any_size_are_none() {
        assert_eq!(svg_dimensions("<svg xmlns=\"x\">"), (None, None));
    }
}
