#[macro_use]
extern crate lazy_static;

extern crate serde_json;
extern crate tera;

use minify_html::{Cfg, minify};
use base64::{Engine as _, engine::{general_purpose}};
use tera::{Context, Tera};

use std::{
    fs::{copy, File},
    io::{prelude::*},
    error::Error,
};

const INPUT_PATH: &str = "src";
const OUTPUT_PATH: &str = "target";
const INDEX_OUTPUT_FILE: &str = "index.html";
const INDEX_TEMPLATE_FILE: &str = "index.tmpl";
const NOT_FOUND_OUTPUT_FILE: &str = "404.html";
const NOT_FOUND_TEMPLATE_FILE: &str = "404.tmpl";
const CONTENT_FILE: &str = "content.json";
const TEMPLATE_PATH: &str = "src/**/*.tmpl";
const STYLE_FILE: &str = "style.css";
const SCRIPT_FILE: &str = "script.js";
const FAVICON_FILE: &str = "favicon.png";

fn concat(p: &str, f: &str) -> String {
    [p, f].join("/")
}

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let mut t = match Tera::new(TEMPLATE_PATH) {
            Ok(t) => t,
            Err(e) => {
                println!("parsing error(s): {}", e);
                ::std::process::exit(1);
            }
        };
        t.autoescape_on(vec![]);
        t
    };
}

lazy_static! {
    pub static ref MINIFIER_CFG: Cfg = {
        let mut cfg = Cfg::new();
        cfg.minify_css = true;
        cfg.minify_js = true;
        cfg.do_not_minify_doctype = true;
        cfg.keep_spaces_between_attributes = true;
        cfg.ensure_spec_compliant_unquoted_attribute_values = true;
        cfg
    };
}

lazy_static! {
    pub static ref CONTENT: serde_json::Value = {
        let mut file = File::open(concat(INPUT_PATH, CONTENT_FILE)).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();
        parsed
    };
}

fn main() {
    let mut ctx = Context::new();
    ctx.insert("title", &CONTENT["title"]);
    ctx.insert("description", &CONTENT["description"]);
    ctx.insert("yandex_metrica", &CONTENT["yandex_metrica"]);
    ctx.insert("canonical", &CONTENT["canonical"]);
    ctx.insert("keywords", &CONTENT["keywords"]);
    ctx.insert("links", &CONTENT["links"]);
    ctx.insert("technologies", &CONTENT["technologies"]);

    let email_base64 = general_purpose::STANDARD.encode(CONTENT["email"].as_str().unwrap().as_bytes());
    ctx.insert("email", &email_base64);

    let mut script = String::new();
    File::open(concat(INPUT_PATH, SCRIPT_FILE)).unwrap().read_to_string(&mut script).unwrap();
    ctx.insert("script", &script);

    let mut style = String::new();
    File::open(concat(INPUT_PATH, STYLE_FILE)).unwrap().read_to_string(&mut style).unwrap();
    ctx.insert("style", &style);

    let render_error = |e: &tera::Error| {
        println!("error: {}", e);
        let mut cause = e.source();
        while let Some(e) = cause {
            println!("reason: {}", e);
            cause = e.source();
        }
    };

    let render_write = |s: &str, f: &str| {
        let minified = minify(&s.as_bytes(), &MINIFIER_CFG);
        let mut file = File::create(concat(OUTPUT_PATH, f)).unwrap();
        file.write_all(&minified).unwrap();
    };

    match TEMPLATES.render(INDEX_TEMPLATE_FILE, &ctx) {
        Ok(s) => render_write(&s, INDEX_OUTPUT_FILE),
        Err(e) => render_error(&e)
    };

    match TEMPLATES.render(NOT_FOUND_TEMPLATE_FILE, &ctx) {
        Ok(s) => render_write(&s, NOT_FOUND_OUTPUT_FILE),
        Err(e) => render_error(&e)
    };

    copy(concat(INPUT_PATH, FAVICON_FILE), concat(OUTPUT_PATH, FAVICON_FILE)).unwrap();
}
