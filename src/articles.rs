use std::{error::Error, fs, path::Path};

use serde::Deserialize;
use toml::value::Datetime;

use crate::markdown::{self, TocItem};

const FRONT_MATTER_DELIMITER: &str = "+++";
const WORDS_PER_MINUTE: usize = 200;
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct FrontMatter {
    pub title: String,
    pub description: String,
    pub date_published: Datetime,
    pub date_modified: Datetime,
    pub tags: Option<Vec<String>>,
    pub lang: Option<String>,
    pub draft: Option<bool>,
}

pub struct Article {
    pub slug: String,
    pub frontmatter: FrontMatter,
    pub body: String,
    pub html: String,
    pub toc: Vec<TocItem>,
    pub diagrams: Vec<markdown::DiagramAsset>,
    pub reading_time: usize,
    pub modified_atom: String,
}

impl Article {
    pub fn url(&self) -> String {
        format!("/blog/{}/", self.slug)
    }

    pub fn lang(&self) -> &str {
        self.frontmatter.lang.as_deref().unwrap_or("en")
    }

    pub fn tags(&self) -> &[String] {
        self.frontmatter.tags.as_deref().unwrap_or(&[])
    }

    pub fn date_display(&self) -> String {
        format_date(&self.frontmatter.date_published)
    }

    pub fn date_iso(&self) -> String {
        iso_date(&self.frontmatter.date_published)
    }

    pub fn date_atom(&self) -> String {
        utc_rfc3339(&self.frontmatter.date_published)
    }

    fn sort_key(&self) -> (u16, u8, u8) {
        date_parts(&self.frontmatter.date_published)
    }
}

fn slug_from_stem(stem: &str) -> String {
    let chars: Vec<char> = stem.chars().collect();
    let date_prefixed = chars.len() > 8
        && chars[0..4].iter().all(|c| c.is_ascii_digit())
        && chars[4] == '-'
        && chars[5..7].iter().all(|c| c.is_ascii_digit())
        && chars[7] == '-';
    if date_prefixed {
        stem[8..].to_owned()
    } else {
        stem.to_owned()
    }
}

pub fn collect_articles(dir: &Path) -> Result<Vec<Article>, Box<dyn Error>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut articles = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        let slug = slug_from_stem(
            path.file_stem()
                .and_then(|s| s.to_str())
                .ok_or("article file name is not valid unicode")?,
        );

        let raw = fs::read_to_string(&path)
            .map_err(|e| format!("failed to read article {slug}: {e}"))?;

        let (frontmatter_source, body) = split_front_matter(&raw)
            .map_err(|e| format!("failed to parse frontmatter of article {slug}: {e}"))?;
        let frontmatter: FrontMatter = toml::from_str(frontmatter_source)
            .map_err(|e| format!("failed to parse frontmatter of article {slug}: {e}"))?;

        if !is_offset_datetime(&frontmatter.date_published)
            || !is_offset_datetime(&frontmatter.date_modified)
        {
            return Err(format!(
                "article {slug}: datePublished and dateModified must be offset date-times like 2026-09-04T08:00:00+03:00"
            ).into());
        }

        if frontmatter.draft.unwrap_or(false) {
            continue;
        }

        let rendered = markdown::render(body, &format!("/blog/{slug}/diagrams"))
            .map_err(|e| format!("failed to render article {slug}: {e}"))?;

        let words = body.split_whitespace().count();
        articles.push(Article {
            slug,
            modified_atom: utc_rfc3339(&frontmatter.date_modified),
            frontmatter,
            body: body.trim().to_owned(),
            html: rendered.html,
            toc: rendered.toc,
            diagrams: rendered.diagrams,
            reading_time: reading_time(words),
        });
    }

    articles.sort_by(|a, b| b.sort_key().cmp(&a.sort_key()).then(a.slug.cmp(&b.slug)));
    Ok(articles)
}

fn split_front_matter(raw: &str) -> Result<(&str, &str), String> {
    let mut start: Option<usize> = None;
    let mut frontmatter_end = 0usize;
    let mut body_start = 0usize;
    let mut offset = 0usize;

    for line in raw.split_inclusive('\n') {
        if line.trim_end_matches(['\r', '\n']) == FRONT_MATTER_DELIMITER {
            if start.is_none() {
                start = Some(offset + line.len());
            } else {
                frontmatter_end = offset;
                body_start = offset + line.len();
                break;
            }
        }
        offset += line.len();
    }

    match (start, frontmatter_end) {
        (Some(start), end) if end > 0 => Ok((&raw[start..end], &raw[body_start..])),
        _ => Err(format!("missing {FRONT_MATTER_DELIMITER} delimited frontmatter block")),
    }
}

pub fn reading_time(words: usize) -> usize {
    (words / WORDS_PER_MINUTE + usize::from(words % WORDS_PER_MINUTE != 0)).max(1)
}

pub fn tag_slug(tag: &str) -> String {
    tag.to_lowercase().replace(' ', "-")
}

pub fn collect_tags(articles: &[Article]) -> Vec<(&str, String, Vec<&Article>)> {
    let mut tags: std::collections::BTreeMap<&str, Vec<&Article>> = std::collections::BTreeMap::new();
    for article in articles {
        for tag in article.tags() {
            tags.entry(tag).or_default().push(article);
        }
    }
    let mut grouped: Vec<_> = tags
        .into_iter()
        .map(|(tag, articles)| (tag, tag_slug(tag), articles))
        .collect();
    grouped.sort_by(|a, b| b.2.len().cmp(&a.2.len()).then(a.1.cmp(&b.1)));
    grouped
}

fn date_parts(datetime: &Datetime) -> (u16, u8, u8) {
    let date = datetime.date.expect("validated calendar date");
    (date.year, date.month, date.day)
}

fn format_date(datetime: &Datetime) -> String {
    let (year, month, day) = date_parts(datetime);
    format!("{} {}, {}", MONTHS[(month - 1) as usize], day, year)
}

fn iso_date(datetime: &Datetime) -> String {
    let (year, month, day) = date_parts(datetime);
    format!("{year:04}-{month:02}-{day:02}")
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if month > 2 { month - 3 } else { month + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn format_utc_secs(secs: i64) -> String {
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

fn is_offset_datetime(datetime: &Datetime) -> bool {
    datetime.date.is_some() && datetime.time.is_some() && datetime.offset.is_some()
}

fn utc_rfc3339(datetime: &Datetime) -> String {
    let date = datetime.date.expect("validated calendar date");
    let time = datetime.time.expect("validated clock time");
    let offset_secs = match datetime.offset {
        Some(toml::value::Offset::Z) => 0,
        Some(toml::value::Offset::Custom { minutes }) => minutes as i64 * 60,
        None => unreachable!("validated UTC offset"),
    };
    let secs = days_from_civil(date.year as i64, date.month as u32, date.day as u32) * 86_400
        + time.hour as i64 * 3600
        + time.minute as i64 * 60
        + time.second as i64
        - offset_secs;
    format_utc_secs(secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "+++\ntitle = \"Hello\"\ndescription = \"First article.\"\ndatePublished = 2026-09-04T08:00:00+03:00\ndateModified = 2026-09-05T09:30:00+03:00\ntags = [\"rust\"]\n+++\n\nBody text.\n";

    #[test]
    fn splits_front_matter_from_body() {
        let (frontmatter, body) = split_front_matter(SAMPLE).unwrap();
        assert!(frontmatter.contains("title = \"Hello\""));
        assert!(!frontmatter.contains("Body text"));
        assert!(body.trim_start().starts_with("Body text."));
    }

    #[test]
    fn splits_front_matter_with_crlf() {
        let crlf = SAMPLE.replace('\n', "\r\n");
        let (frontmatter, body) = split_front_matter(&crlf).unwrap();
        assert!(frontmatter.contains("title = \"Hello\""));
        assert!(body.trim_start().starts_with("Body text."));
    }

    #[test]
    fn missing_delimiter_is_an_error() {
        assert!(split_front_matter("no frontmatter here").is_err());
    }

    #[test]
    fn parses_full_front_matter() {
        let (source, _) = split_front_matter(SAMPLE).unwrap();
        let fm: FrontMatter = toml::from_str(source).unwrap();
        assert_eq!(fm.title, "Hello");
        assert_eq!(fm.tags.as_deref(), Some(&["rust".to_owned()][..]));
        assert_eq!(fm.lang, None);
        assert_eq!(fm.draft, None);
    }

    #[test]
    fn unknown_front_matter_field_is_rejected() {
        let raw = "+++\ntitle = \"x\"\ndescription = \"y\"\ndatePublished = 2026-09-04T08:00:00+03:00\ndateModified = 2026-09-04T09:00:00+03:00\nsurprise = 1\n+++\n";
        let (source, _) = split_front_matter(raw).unwrap();
        assert!(toml::from_str::<FrontMatter>(source).is_err());
    }

    #[test]
    fn missing_required_field_is_rejected() {
        let raw = "+++\ntitle = \"x\"\ndescription = \"y\"\n+++\n";
        let (source, _) = split_front_matter(raw).unwrap();
        assert!(toml::from_str::<FrontMatter>(source).is_err());
    }

    #[test]
    fn formats_display_and_iso_dates() {
        let fm: FrontMatter = toml::from_str(
            "title = \"x\"\ndescription = \"y\"\ndatePublished = 2026-09-04T08:00:00+03:00\ndateModified = 2026-09-05T09:30:00+03:00",
        )
        .unwrap();
        assert_eq!(format_date(&fm.date_published), "Sep 4, 2026");
        assert_eq!(iso_date(&fm.date_published), "2026-09-04");
        assert_eq!(fm.date_published.to_string(), "2026-09-04T08:00:00+03:00");
    }

    #[test]
    fn utc_rfc3339_normalizes_offsets_to_utc() {
        let plus3 = toml::from_str::<FrontMatter>(
            "title = \"x\"\ndescription = \"y\"\ndatePublished = 2026-09-04T00:00:00+03:00\ndateModified = 2026-09-04T01:00:00+03:00",
        )
        .unwrap();
        assert_eq!(utc_rfc3339(&plus3.date_published), "2026-09-03T21:00:00Z");

        let utc = toml::from_str::<FrontMatter>(
            "title = \"x\"\ndescription = \"y\"\ndatePublished = 2026-09-04T08:00:00Z\ndateModified = 2026-09-04T09:00:00Z",
        )
        .unwrap();
        assert_eq!(utc_rfc3339(&utc.date_published), "2026-09-04T08:00:00Z");

        let minus8 = toml::from_str::<FrontMatter>(
            "title = \"x\"\ndescription = \"y\"\ndatePublished = 2026-09-04T23:30:00-08:00\ndateModified = 2026-09-05T00:30:00-08:00",
        )
        .unwrap();
        assert_eq!(utc_rfc3339(&minus8.date_published), "2026-09-05T07:30:00Z");
    }

    #[test]
    fn civil_conversions_round_trip() {
        for days in [-100_000i64, -1, 0, 1, 19_000, 100_000] {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m as u32, d as u32), days);
        }
    }

    #[test]
    fn date_only_or_naive_datetimes_are_rejected() {
        let dir = std::env::temp_dir().join("main-tests-articles-dates");
        fs::create_dir_all(&dir).unwrap();
        let valid_modified = "dateModified = 2026-09-04T09:00:00+03:00";
        let valid_published = "datePublished = 2026-09-04T08:00:00+03:00";
        for (name, frontmatter) in [
            ("date_only.md", format!("datePublished = 2026-09-04\n{valid_modified}")),
            ("naive.md", format!("{valid_published}\ndateModified = 2026-09-04T09:00:00")),
            ("naive_modified.md", format!("{valid_published}\ndateModified = 2026-09-04T09:00:00")),
        ] {
            fs::write(
                dir.join(name),
                format!("+++\ntitle = \"{name}\"\ndescription = \"d\"\n{frontmatter}\n+++\n\nBody."),
            )
            .unwrap();
        }
        let err = match collect_articles(&dir) {
            Err(e) => e.to_string(),
            Ok(_) => panic!("date-only and naive datetimes must be rejected"),
        };
        assert!(err.contains("must be offset date-times"), "{err}");
    }

    #[test]
    fn reading_time_rounds_up_and_never_zero() {
        assert_eq!(reading_time(0), 1);
        assert_eq!(reading_time(1), 1);
        assert_eq!(reading_time(200), 1);
        assert_eq!(reading_time(201), 2);
        assert_eq!(reading_time(400), 2);
    }

    #[test]
    fn slug_drops_date_prefix_only() {
        assert_eq!(slug_from_stem("2026-09-my-article"), "my-article");
        assert_eq!(slug_from_stem("hello-world"), "hello-world");
        assert_eq!(slug_from_stem("2026-1-my-article"), "2026-1-my-article");
    }

    #[test]
    fn collect_sorts_newest_first_and_skips_drafts() {
        let dir = std::env::temp_dir().join("main-tests-articles");
        fs::create_dir_all(&dir).unwrap();
        for (name, date) in [
            ("old.md", "2026-01-01T00:00:00+03:00"),
            ("new.md", "2026-08-31T00:00:00+03:00"),
        ] {
            fs::write(
                dir.join(name),
                format!("+++\ntitle = \"{name}\"\ndescription = \"d\"\ndatePublished = {date}\ndateModified = {date}\n+++\n\nBody."),
            )
            .unwrap();
        }
        fs::write(
            dir.join("draft.md"),
            "+++\ntitle = \"d\"\ndescription = \"d\"\ndatePublished = 2026-07-01T00:00:00+03:00\ndateModified = 2026-07-01T00:00:00+03:00\ndraft = true\n+++\n\nBody.",
        )
        .unwrap();

        let articles = collect_articles(&dir).unwrap();
        let slugs: Vec<_> = articles.iter().map(|p| p.slug.as_str()).collect();
        assert_eq!(slugs, ["new", "old"]);
        assert_eq!(articles[0].url(), "/blog/new/");
    }
}
