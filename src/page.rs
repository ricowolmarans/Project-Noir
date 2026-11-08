use eframe::egui::Color32;
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use std::io::Read;

pub const ENGINES: [(&str, &str); 2] = [
    ("Google", "https://www.google.com/search?q="),
    ("DuckDuckGo Lite", "https://lite.duckduckgo.com/lite/?q="),
];

const BLOCKS: &[&str] = &[
    "p", "div", "li", "ul", "ol", "tr", "table", "section", "article", "header", "footer", "nav", "main", "form",
    "blockquote", "pre", "h1", "h2", "h3", "h4", "h5", "h6", "center", "dl", "dt", "dd", "figure", "aside", "body", "html",
];

#[derive(Clone)]
pub struct Run {
    pub text: String,
    pub color: Option<Color32>,
    pub bold: bool,
    pub size: f32,
    pub link: Option<String>,
}

#[derive(Clone)]
pub struct Block {
    pub runs: Vec<Run>,
    pub center: bool,
    pub bg: Option<Color32>,
    pub mt: f32,
}

pub struct Page {
    pub title: String,
    pub bg: Option<Color32>,
    pub blocks: Vec<Block>,
}

#[derive(Clone)]
struct Style {
    color: Option<Color32>,
    bold: bool,
    size: f32,
    center: bool,
    bg: Option<Color32>,
    mt: f32,
    mb: f32,
    link: Option<String>,
}

impl Style {
    fn base() -> Self {
        Style { color: None, bold: false, size: 15.0, center: false, bg: None, mt: 0.0, mb: 0.0, link: None }
    }
}

struct W {
    out: Vec<Block>,
    runs: Vec<Run>,
    text: String,
    pend: f32,
    title: String,
    page_bg: Option<Color32>,
    base: String,
}

impl W {
    fn end_run(&mut self, st: &Style) {
        if self.text.is_empty() {
            return;
        }
        let t = std::mem::take(&mut self.text);
        self.runs.push(Run { text: t, color: st.color, bold: st.bold, size: st.size, link: st.link.clone() });
    }

    fn flush(&mut self, st: &Style, bg: Option<Color32>) {
        self.end_run(st);
        if let Some(f) = self.runs.first_mut() {
            f.text = f.text.trim_start().to_string();
        }
        if let Some(l) = self.runs.last_mut() {
            l.text = l.text.trim_end().to_string();
        }
        self.runs.retain(|r| !r.text.is_empty());
        if self.runs.is_empty() {
            return;
        }
        if self.out.len() < 4000 {
            let runs = std::mem::take(&mut self.runs);
            self.out.push(Block { runs, center: st.center, bg, mt: self.pend });
        } else {
            self.runs.clear();
        }
        self.pend = 0.0;
    }

    fn add_text(&mut self, s: &str) {
        for ch in s.chars() {
            if ch.is_whitespace() {
                let prev_space = if self.text.is_empty() {
                    self.runs.last().map(|r| r.text.ends_with(' ')).unwrap_or(true)
                } else {
                    self.text.ends_with(' ')
                };
                if !prev_space {
                    self.text.push(' ');
                }
            } else {
                self.text.push(ch);
            }
        }
    }

    fn walk(&mut self, h: &Handle, st: &Style, bg: Option<Color32>) {
        match &h.data {
            NodeData::Document => {
                for c in h.children.borrow().iter() {
                    self.walk(c, st, bg);
                }
            }
            NodeData::Text { contents } => {
                let t = contents.borrow();
                let s: &str = &t;
                self.add_text(s);
            }
            NodeData::Element { name, attrs, .. } => {
                let tag: &str = &name.local;
                if matches!(tag, "script" | "style" | "noscript" | "template" | "svg" | "iframe" | "object") {
                    return;
                }
                if tag == "head" {
                    for c in h.children.borrow().iter() {
                        if let NodeData::Element { name, .. } = &c.data {
                            if &*name.local == "title" {
                                self.title = text_of(c).split_whitespace().collect::<Vec<_>>().join(" ");
                            }
                        }
                    }
                    return;
                }
                let get = |n: &str| -> Option<String> {
                    attrs.borrow().iter().find(|a| &*a.name.local == n).map(|a| a.value.to_string())
                };
                if get("hidden").is_some() {
                    return;
                }
                if tag == "br" {
                    self.end_run(st);
                    self.flush(st, bg);
                    return;
                }
                if tag == "hr" {
                    self.end_run(st);
                    self.flush(st, bg);
                    self.text.push_str("────────────────────");
                    self.end_run(st);
                    self.flush(st, bg);
                    return;
                }
                if tag == "img" {
                    if let Some(alt) = get("alt") {
                        let a = alt.trim();
                        if !a.is_empty() {
                            self.add_text(&format!("[{a}] "));
                        }
                    }
                    return;
                }

                let is_block = BLOCKS.contains(&tag);
                let mut s = st.clone();
                s.bg = None;
                s.mt = 0.0;
                s.mb = 0.0;
                ua(&mut s, tag);
                if tag == "a" {
                    if let Some(l) = get("href").and_then(|x| resolve(&self.base, &x)) {
                        s.link = Some(l);
                        s.color = Some(Color32::from_rgb(130, 170, 255));
                    }
                }
                if let Some(inl) = get("style") {
                    for (k, v) in parse_decls(&inl) {
                        if apply(&mut s, &k, &v, st.size) {
                            return;
                        }
                    }
                }
                if (tag == "body" || tag == "html") && s.bg.is_some() {
                    self.page_bg = s.bg;
                    s.bg = None;
                }

                self.end_run(st);
                if is_block {
                    self.flush(st, bg);
                    self.pend = self.pend.max(s.mt);
                }
                let child_bg = s.bg.or(bg);
                if tag == "li" {
                    self.add_text("• ");
                }
                for c in h.children.borrow().iter() {
                    self.walk(c, &s, child_bg);
                }
                self.end_run(&s);
                if is_block {
                    self.flush(&s, child_bg);
                    self.pend = self.pend.max(s.mb);
                }
            }
            _ => {}
        }
    }
}

fn text_of(h: &Handle) -> String {
    let mut s = String::new();
    if let NodeData::Text { contents } = &h.data {
        s.push_str(&contents.borrow());
    }
    for c in h.children.borrow().iter() {
        s.push_str(&text_of(c));
    }
    s
}

pub fn load_page(url: &str) -> Result<Page, String> {
    let html = fetch(url)?;
    let mut bytes = html.as_bytes();
    let dom = parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut bytes)
        .map_err(|e| e.to_string())?;
    let mut w = W { out: vec![], runs: vec![], text: String::new(), pend: 0.0, title: String::new(), page_bg: None, base: url.to_string() };
    let base = Style::base();
    w.walk(&dom.document, &base, None);
    w.flush(&base, None);
    Ok(Page { title: w.title, bg: w.page_bg, blocks: w.out })
}

fn fetch(url: &str) -> Result<String, String> {
    let resp = ureq::get(url)
        .set("User-Agent", "Noir/0.1.0 (beta)")
        .timeout(std::time::Duration::from_secs(15))
        .call()
        .map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    resp.into_reader().take(5_000_000).read_to_end(&mut buf).map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
                (b as char).to_string()
            } else if b == b' ' {
                "+".to_string()
            } else {
                format!("%{:02X}", b)
            }
        })
        .collect()
}

pub fn to_url(input: &str, engine: usize) -> String {
    let s = input.trim();
    if s.starts_with("http://") || s.starts_with("https://") {
        s.to_string()
    } else if s.contains('.') && !s.contains(' ') {
        format!("https://{s}")
    } else {
        format!("{}{}", ENGINES[engine].1, enc(s))
    }
}

fn resolve(base: &str, href: &str) -> Option<String> {
    let h = href.trim();
    if h.is_empty() || h.starts_with('#') || h.starts_with("javascript:") || h.starts_with("mailto:") || h.starts_with("data:") {
        return None;
    }
    if h.starts_with("http://") || h.starts_with("https://") {
        return Some(h.to_string());
    }
    if let Some(r) = h.strip_prefix("//") {
        return Some(format!("https://{r}"));
    }
    let after = base.find("://")? + 3;
    let origin_end = base[after..].find('/').map(|p| p + after).unwrap_or(base.len());
    if h.starts_with('/') {
        return Some(format!("{}{}", &base[..origin_end], h));
    }
    let dir_end = base[after..].rfind('/').map(|p| p + after).unwrap_or(base.len());
    Some(format!("{}/{}", &base[..dir_end], h))
}

fn parse_decls(b: &str) -> Vec<(String, String)> {
    b.split(';')
        .filter_map(|d| {
            let (k, v) = d.split_once(':')?;
            Some((k.trim().to_lowercase(), v.trim().to_string()))
        })
        .collect()
}

fn parse_color(s: &str) -> Option<Color32> {
    let s = s.trim().to_lowercase();
    if let Some(h) = s.strip_prefix('#') {
        let v: Vec<u8> = h.chars().map(|c| c.to_digit(16).map(|d| d as u8)).collect::<Option<Vec<u8>>>()?;
        return match v.len() {
            3 => Some(Color32::from_rgb(v[0] * 17, v[1] * 17, v[2] * 17)),
            6 => Some(Color32::from_rgb(v[0] * 16 + v[1], v[2] * 16 + v[3], v[4] * 16 + v[5])),
            _ => None,
        };
    }
    if let Some(r) = s.strip_prefix("rgb") {
        let inner = r.trim_start_matches('a').trim_start_matches('(').split(')').next()?;
        let n: Vec<f32> = inner.split(|c: char| c == ',' || c == ' ' || c == '/').filter(|x| !x.is_empty()).filter_map(|x| x.parse().ok()).collect();
        if n.len() >= 3 {
            return Some(Color32::from_rgb(n[0] as u8, n[1] as u8, n[2] as u8));
        }
        return None;
    }
    match s.as_str() {
        "black" => Some(Color32::BLACK),
        "white" => Some(Color32::WHITE),
        "red" => Some(Color32::from_rgb(255, 0, 0)),
        "green" => Some(Color32::from_rgb(0, 128, 0)),
        "blue" => Some(Color32::from_rgb(0, 0, 255)),
        "yellow" => Some(Color32::from_rgb(255, 255, 0)),
        "gray" | "grey" => Some(Color32::from_rgb(128, 128, 128)),
        "silver" => Some(Color32::from_rgb(192, 192, 192)),
        "orange" => Some(Color32::from_rgb(255, 165, 0)),
        "purple" => Some(Color32::from_rgb(128, 0, 128)),
        "pink" => Some(Color32::from_rgb(255, 192, 203)),
        "navy" => Some(Color32::from_rgb(0, 0, 128)),
        "teal" => Some(Color32::from_rgb(0, 128, 128)),
        "lime" => Some(Color32::from_rgb(0, 255, 0)),
        "maroon" => Some(Color32::from_rgb(128, 0, 0)),
        "cyan" | "aqua" => Some(Color32::from_rgb(0, 255, 255)),
        "magenta" | "fuchsia" => Some(Color32::from_rgb(255, 0, 255)),
        _ => None,
    }
}

fn parse_len(v: &str, base: f32) -> Option<f32> {
    let v = v.trim();
    let num: String = v.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    let n: f32 = num.parse().ok()?;
    let unit = &v[num.len()..];
    Some(match unit {
        "em" => n * base,
        "rem" => n * 16.0,
        "%" => base * n / 100.0,
        "pt" => n * 1.333,
        _ => n,
    })
}

// returns true when the element should be hidden
fn apply(s: &mut Style, k: &str, v: &str, psize: f32) -> bool {
    let v = v.trim().trim_end_matches("!important").trim();
    match k {
        "color" => {
            if let Some(c) = parse_color(v) {
                s.color = Some(c);
            }
        }
        "background" | "background-color" => {
            if let Some(c) = parse_color(v).or_else(|| v.split_whitespace().find_map(parse_color)) {
                s.bg = Some(c);
            }
        }
        "font-size" => {
            if let Some(n) = parse_len(v, psize) {
                s.size = n.clamp(8.0, 72.0);
            }
        }
        "font-weight" => {
            s.bold = v == "bold" || v == "bolder" || v.parse::<u32>().map(|n| n >= 600).unwrap_or(false);
        }
        "text-align" => {
            s.center = v == "center";
        }
        "display" | "visibility" => {
            if v == "none" || v == "hidden" {
                return true;
            }
        }
        "margin-top" => {
            if let Some(n) = parse_len(v, 15.0) {
                s.mt = n.clamp(0.0, 60.0);
            }
        }
        "margin-bottom" => {
            if let Some(n) = parse_len(v, 15.0) {
                s.mb = n.clamp(0.0, 60.0);