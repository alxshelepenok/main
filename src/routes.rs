pub const SLICE_PAGE: &str = "page";
pub const SLICE_PERSON: &str = "person";

pub struct Route<'a> {
    pub path: &'a str,
}

impl Route<'static> {
    pub fn home() -> Self {
        Self { path: "/" }
    }

    pub fn blog() -> Self {
        Self { path: "/blog/" }
    }

    pub fn tags_hub() -> Self {
        Self { path: "/blog/tags/" }
    }
}

impl<'a> Route<'a> {
    pub fn new(path: &'a str) -> Self {
        Self { path }
    }

    pub fn href(&self, slice: &str) -> String {
        format!("{}#{slice}", self.path)
    }

    pub fn id(&self, canonical: &str, slice: &str) -> String {
        let base = canonical.trim_end_matches('/');
        format!("{base}{}#{slice}", self.path)
    }

    #[allow(unused)]
    pub fn canonical(&self, base: &str) -> String {
        let base = base.trim_end_matches('/');
        format!("{base}{}", self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CANONICAL: &str = "https://example.com";

    #[test]
    fn home_route_forms_match_the_root_id_shape() {
        let home = Route::home();
        assert_eq!(home.href(SLICE_PAGE), "/#page");
        assert_eq!(home.id(CANONICAL, SLICE_PERSON), "https://example.com/#person");
        assert_eq!(home.canonical(CANONICAL), "https://example.com/");
    }

    #[test]
    fn page_route_forms_carry_the_path() {
        let article = Route::new("/blog/light-cone/");
        assert_eq!(article.href(SLICE_PAGE), "/blog/light-cone/#page");
        assert_eq!(article.id(CANONICAL, SLICE_PAGE), "https://example.com/blog/light-cone/#page");
        assert_eq!(article.canonical(CANONICAL), "https://example.com/blog/light-cone/");
    }

    #[test]
    fn canonical_with_trailing_slash_stays_stable() {
        let blog = Route::blog();
        assert_eq!(blog.id("https://example.com/", SLICE_PAGE), "https://example.com/blog/#page");
    }
}
