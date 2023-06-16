use std::error::Error;
use std::sync::LazyLock;

use merman::render::{
    CompiledHostTheme, HeadlessRenderer, HostThemeOutput, HostThemeProfile,
    HostThemeRootBackground,
};
use serde_json::json;

static SITE_THEME: LazyLock<CompiledHostTheme> = LazyLock::new(|| {
    HostThemeProfile::builder()
        .site_config("theme", json!("base"))
        .site_config(
            "themeCSS",
            json!(
                ".edgeLabel, .edgeLabel p, .labelBkg { border-radius: 6px; } \
                 .edgeLabel p { padding: 3px 6px; }"
            ),
        )
        .site_config(
            "themeVariables",
            json!({
                "background": "#c5c0ce",
                "primaryColor": "#d2ceda",
                "primaryBorderColor": "#0d0b0f",
                "primaryTextColor": "#0d0b0f",
                "secondaryColor": "#b7b1c3",
                "secondaryBorderColor": "#51495f",
                "secondaryTextColor": "#0d0b0f",
                "tertiaryColor": "#a8a0b6",
                "tertiaryBorderColor": "#51495f",
                "tertiaryTextColor": "#0d0b0f",
                "lineColor": "#51495f",
                "textColor": "#0d0b0f",
                "edgeLabelBackground": "#d2ceda",
                "clusterBkg": "#b7b1c3",
                "clusterBorder": "#51495f",
                "noteBkgColor": "#d2ceda",
                "noteBorderColor": "#51495f",
                "noteTextColor": "#0d0b0f"
            }),
        )
        .output(HostThemeOutput {
            root_background: HostThemeRootBackground::Color("#c5c0ce".to_owned()),
            ..HostThemeOutput::default()
        })
        .build()
        .compile()
});

const EDGE_LABEL_PAD_X: f64 = 6.0;
const EDGE_LABEL_PAD_Y: f64 = 3.0;
const EDGE_LABELS_OPEN: &str = "<g class=\"edgeLabels\">";
const NODES_OPEN: &str = "<g class=\"nodes\">";
const LABEL_MARKER: &str = "<g class=\"label\"";

pub fn render_svg(source: &str) -> Result<String, Box<dyn Error>> {
    let svg = HeadlessRenderer::new()
        .with_compiled_host_theme(&SITE_THEME)
        .render_svg_sync(source)?
        .ok_or("no mermaid diagram detected in source")?;
    Ok(pad_edge_labels(&svg))
}

fn fmt_num(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

fn pad_edge_labels(svg: &str) -> String {
    let Some(start) = svg.find(EDGE_LABELS_OPEN) else {
        return svg.to_owned();
    };
    let Some(offset) = svg[start..].find(NODES_OPEN) else {
        return svg.to_owned();
    };
    let end = start + offset;

    let mut out = String::with_capacity(svg.len() + 96);
    out.push_str(&svg[..start]);
    out.push_str(&pad_label_slice(&svg[start..end]));
    out.push_str(&svg[end..]);
    out
}

fn pad_label_slice(slice: &str) -> String {
    let mut out = String::with_capacity(slice.len() + 64);
    let mut rest = slice;
    while let Some(pos) = rest.find(LABEL_MARKER) {
        let (head, tail) = rest.split_at(pos);
        out.push_str(head);
        match rewrite_label(tail) {
            Some((rewritten, consumed)) => {
                out.push_str(&rewritten);
                rest = &tail[consumed..];
            }
            None => {
                out.push_str(LABEL_MARKER);
                rest = &tail[LABEL_MARKER.len()..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn rewrite_label(src: &str) -> Option<(String, usize)> {
    let t0 = src.find("translate(")? + "translate(".len();
    let t1 = src[t0..].find(')')? + t0;
    let (tx, ty) = src[t0..t1].split_once(',')?;
    let tx: f64 = tx.trim().parse().ok()?;
    let ty: f64 = ty.trim().parse().ok()?;

    let w0 = src[t1..].find("width=\"")? + t1 + "width=\"".len();
    let w1 = src[w0..].find('"')? + w0;
    let w: f64 = src[w0..w1].parse().ok()?;

    let h0 = src[w1..].find("height=\"")? + w1 + "height=\"".len();
    let h1 = src[h0..].find('"')? + h0;
    let h: f64 = src[h0..h1].parse().ok()?;

    let mut out = String::with_capacity(h1 + 1);
    out.push_str(&src[..t0]);
    out.push_str(&fmt_num(tx - EDGE_LABEL_PAD_X));
    out.push(',');
    out.push_str(&fmt_num(ty - EDGE_LABEL_PAD_Y));
    out.push_str(&src[t1..w0]);
    out.push_str(&fmt_num(w + 2.0 * EDGE_LABEL_PAD_X));
    out.push_str(&src[w1..h0]);
    out.push_str(&fmt_num(h + 2.0 * EDGE_LABEL_PAD_Y));
    out.push_str(&src[h1..h1 + 1]);
    Some((out, h1 + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_flowchart_to_svg() {
        let svg = render_svg("flowchart TD\nA[Start] --> B[Done]").unwrap();
        assert!(svg.trim_start().to_lowercase().starts_with("<svg"));
    }

    #[test]
    fn site_theme_applies_pale_slate_palette() {
        let svg = render_svg("flowchart TD\nA[Start] --> B[Done]").unwrap();
        assert!(svg.contains("background-color: #c5c0ce"), "root background");
        assert!(svg.contains("#d2ceda"), "node fill");
        assert!(!svg.contains("background-color: white"));
    }

    #[test]
    fn init_directive_overrides_site_theme() {
        let md = "%%{init: {\"themeVariables\": {\"lineColor\": \"#123456\"}}}%%\nflowchart TD\nA --> B";
        let svg = render_svg(md).unwrap();
        assert!(svg.contains("flowchart-link{stroke:#123456"));
    }

    #[test]
    fn edge_label_chips_are_rounded_and_padded() {
        let svg = render_svg("flowchart TD\nA ==>|blocks| B").unwrap();
        assert!(
            svg.contains("#merman .edgeLabel, #merman .edgeLabel p, #merman .labelBkg { border-radius: 6px; }"),
            "every layer that paints the chip must round"
        );
        assert!(svg.contains("#merman .edgeLabel p { padding: 3px 6px; }"));
    }

    #[test]
    fn pad_edge_labels_grows_only_edge_label_boxes() {
        let svg = concat!(
            "<svg><g class=\"edgePaths\"/><g class=\"edgeLabels\">",
            "<g class=\"edgeLabel\" transform=\"translate(100,200)\">",
            "<g class=\"label\" data-id=\"L_A_B_0\" transform=\"translate(-22.5,-12)\">",
            "<foreignObject width=\"45\" height=\"24\"><div class=\"labelBkg\">x</div></foreignObject></g></g>",
            "</g><g class=\"nodes\">",
            "<g class=\"label\" data-id=\"A\" transform=\"translate(-22.5,-12)\">",
            "<foreignObject width=\"45\" height=\"24\">node</foreignObject></g>",
            "</g></svg>"
        );
        let out = pad_edge_labels(svg);
        assert!(out.contains("translate(-28.5,-15)\""));
        assert!(out.contains("width=\"57\" height=\"30\""));
        let nodes = &out[out.find(NODES_OPEN).unwrap()..];
        assert!(nodes.contains("translate(-22.5,-12)\""));
        assert!(nodes.contains("width=\"45\" height=\"24\""));
    }

    #[test]
    fn padded_edge_labels_stay_centered() {
        let svg = render_svg("flowchart TD\nA ==>|blocks| B").unwrap();
        let start = svg.find(EDGE_LABELS_OPEN).unwrap();
        let end = svg[start..].find(NODES_OPEN).unwrap() + start;
        let slice = &svg[start..end];

        let mut checked = 0;
        let mut rest = slice;
        while let Some(pos) = rest.find("translate(-") {
            let t0 = pos + "translate(".len();
            let t1 = rest[t0..].find(')').unwrap() + t0;
            let (tx, ty) = rest[t0..t1].split_once(',').unwrap();
            let tx: f64 = tx.parse().unwrap();
            let ty: f64 = ty.parse().unwrap();

            let w0 = rest[t1..].find("width=\"").unwrap() + t1 + "width=\"".len();
            let w1 = rest[w0..].find('"').unwrap() + w0;
            let w: f64 = rest[w0..w1].parse().unwrap();

            let h0 = rest[w1..].find("height=\"").unwrap() + w1 + "height=\"".len();
            let h1 = rest[h0..].find('"').unwrap() + h0;
            let h: f64 = rest[h0..h1].parse().unwrap();

            assert!(
                (tx + w / 2.0).abs() < 0.001 && (ty + h / 2.0).abs() < 0.001,
                "label must stay centered: translate({tx},{ty}) for {w}x{h}"
            );
            assert!((h - 30.0).abs() < f64::EPSILON, "single-line label height is 24+6");
            checked += 1;
            rest = &rest[h1..];
        }
        assert!(checked > 0, "no edge labels found to verify");
    }
}
