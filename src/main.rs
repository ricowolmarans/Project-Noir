use eframe::egui::{self, Color32, RichText};
use std::sync::mpsc::{channel, Receiver, Sender};

const ENGINES: [(&str, &str); 2] = [
    ("Google", "https://www.google.com/search?q="),
    ("DuckDuckGo Lite", "https://lite.duckduckgo.com/lite/?q="),
];

const BLOCKS: &[&str] = &[
    "p", "div", "br", "hr", "li", "ul", "ol", "tr", "table", "section", "article", "header", "footer", "nav", "main",
    "form", "blockquote", "pre", "h1", "h2", "h3", "h4", "h5", "h6", "center", "dl", "dt", "dd", "figure", "aside",
];
const VOID: &[&str] = &["br", "hr", "img", "meta", "link", "input", "area", "base", "col", "embed", "source", "track", "wbr", "param"];

#[derive(Clone)]
struct Style {
    color: Option<Color32>,
    bold: bool,
    size: f32,
    center: bool,
    hidden: bool,
    bg: Option<Color32>,
    mt: f32,
    mb: f32,
    link: Option<String>,
}

impl Style {
    fn base() -> Self {
        Style { color: None, bold: false, size: 15.0, center: false, hidden: false, bg: None, mt: 0.0, mb: 0.0, link: None }
    }
}

struct Frame {
    tag: String,
    style: Style,
}

#[derive(Clone)]
struct Run {
    text: String,
    color: Option<Color32>,
    bold: bool,
    size: f32,
    link: Option<String>,
}

#[derive(Clone)]
struct Block {
    runs: Vec<Run>,
    center: bool,
    bg: Option<Color32>,
    mt: f32,
}

struct Page {
    title: String,
    bg: Option<Color32>,
    blocks: Vec<Block>,
}

type Msg = (u64, String, Result<Page, String>);

struct Tab {
    id: u64,
    title: String,
    url: String,
    blocks: Vec<Block>,
    bg: Option<Color32>,
    asleep: bool,
    loading: bool,
    err: Option<String>,
}

impl Tab {
    fn new(id: u64) -> Self {
        Tab { id, title: "New tab".into(), url: String::new(), blocks: vec![], bg: None, asleep: false, loading: false, err: None }
    }
}

struct Noir {
    tabs: Vec<Tab>,
    cur: usize,
    next_id: u64,
    addr: String,
    game: bool,
    hist: Vec<String>,
    unlocked: bool,
    pw: String,
    pw_err: bool,
    panel: u8,
    engine: usize,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
}

impl Noir {
    fn new() -> Self {
        let (tx, rx) = channel();
        Noir { tabs: vec![Tab::new(1)], cur: 0, next_id: 2, addr: String::new(), game: false, hist: vec![], unlocked: false, pw: String::new(), pw_err: false, panel: 0, engine: 0, tx, rx }
    }

    fn load(&mut self, idx: usize, url: String, ctx: &egui::Context) {
        let t = &mut self.tabs[idx];
        t.url = url.clone();
        t.loading = true;
        t.err = None;
        t.asleep = false;
        let id = t.id;
        if idx == self.cur {
            self.addr = url.clone();
        }
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let r = load_page(&url);
            let _ = tx.send((id, url, r));
            ctx.request_repaint();
        });
    }

    fn go(&mut self, input: &str, ctx: &egui::Context) {
        let url = to_url(input, self.engine);
        let c = self.cur;
        self.load(c, url, ctx);
    }

    fn sleep_others(&mut self) {
        if !self.game {
            return;
        }
        for (i, t) in self.tabs.iter_mut().enumerate() {
            if i != self.cur && !t.url.is_empty() {
                t.blocks = Vec::new();
                t.asleep = true;
            }
        }
    }

    fn switch(&mut self, i: usize, ctx: &egui::Context) {
        self.cur = i;
        self.addr = self.tabs[i].url.clone();
        if self.tabs[i].asleep {
            let u = self.tabs[i].url.clone();
            self.load(i, u, ctx);
        }
        self.sleep_others();
    }

    fn new_tab(&mut self, ctx: &egui::Context) {
        let id = self.next_id;
        self.next_id += 1;
        self.tabs.push(Tab::new(id));
        let n = self.tabs.len() - 1;
        self.switch(n, ctx);
    }
}

impl eframe::App for Noir {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok((id, url, r)) = self.rx.try_recv() {
            if let Some(t) = self.tabs.iter_mut().find(|t| t.id == id) {
                if t.url != url {
                    continue;
                }
                t.loading = false;
                match r {
                    Ok(p) => {
                        t.title = if p.title.is_empty() { url.clone() } else { p.title };
                        t.blocks = p.blocks;
                        t.bg = p.bg;
                        self.hist.push(url);
                    }
                    Err(e) => t.err = Some(e),
                }
            }
        }

        let mut sw = None;
        let mut cl = None;
        let mut add = false;
        egui::SidePanel::left("tabs").exact_width(210.0).show(ctx, |ui| {
            ui.heading(RichText::new("Noir").size(26.0).strong());
            ui.add_space(8.0);
            for (i, t) in self.tabs.iter().enumerate() {
                ui.horizontal(|ui| {
                    let label = format!("{}{}", if t.asleep { "zzz " } else { "" }, t.title.chars().take(20).collect::<String>());
                    if ui.selectable_label(i == self.cur, label).clicked() {
                        sw = Some(i);
                    }
                    if ui.small_button("x").clicked() {
                        cl = Some(i);
                    }
                });
            }
            if ui.button("+ New tab").clicked() {
                add = true;
            }
        });
        if add {
            self.new_tab(ctx);
        }
        if let Some(i) = cl {
            self.tabs.remove(i);
            if self.tabs.is_empty() {
                let id = self.next_id;
                self.next_id += 1;
                self.tabs.push(Tab::new(id));
            }
            if self.cur >= self.tabs.len() {
                self.cur = self.tabs.len() - 1;
            } else if i < self.cur {
                self.cur -= 1;
            }
            let c = self.cur;
            self.switch(c, ctx);
        }
        if let Some(i) = sw {
            self.switch(i, ctx);
        }

        let mut go_to: Option<String> = None;
        egui::TopBottomPanel::top("bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let w = (ui.available_width() - 170.0).max(80.0);
                let hint = format!("Search {} or type a URL", ENGINES[self.engine].0);
                let r = ui.add(egui::TextEdit::singleline(&mut self.addr).desired_width(w).hint_text(hint));
                if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    go_to = Some(self.addr.clone());
                }
                if ui.button("History").clicked() {
                    self.panel = 1;
                }
                if ui.button("Settings").clicked() {
                    self.panel = 2;
                }
            });
        });

        let asleep = self.tabs.iter().filter(|t| t.asleep).count();
        let mut toggled = false;
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.checkbox(&mut self.game, "Gaming mode (sleeps background tabs)").changed() {
                    toggled = true;
                }
                ui.label(format!("{} asleep", asleep));
            });
        });
        if toggled {
            self.sleep_others();
        }

        let mut nav: Option<String> = None;
        let panel_bg = self.tabs[self.cur].bg.unwrap_or(Color32::BLACK);
        egui::CentralPanel::default().frame(egui::Frame::none().fill(panel_bg).inner_margin(12.0)).show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let t = &self.tabs[self.cur];
                if t.url.is_empty() {
                    ui.add_space(80.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("Noir").size(72.0).strong());
                        ui.label("Private by design. Type in the bar above.");
                    });
                    return;
                }
                if t.loading {
                    ui.spinner();
                }
                if let Some(e) = &t.err {
                    ui.label(format!("Could not load this page: {e}"));
                }
                for b in &t.blocks {
                    draw(ui, b, panel_bg, &mut nav);
                }
            });
        });
        if let Some(s) = go_to {
            if !s.trim().is_empty() {
                self.go(s.trim(), ctx);
            }
        }
        if let Some(u) = nav {
            let c = self.cur;
            self.load(c, u, ctx);
        }

        if self.panel != 0 {
            let kind = self.panel;
            let mut open = true;
            egui::Window::new(if kind == 1 { "History" } else { "Settings" }).open(&mut open).show(ctx, |ui| {
                if !self.unlocked {
                    ui.label("Enter your password (beta placeholder: noir)");
                    let r = ui.add(egui::TextEdit::singleline(&mut self.pw).password(true));
                    let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ui.button("Unlock").clicked() || enter {
                        self.pw_err = self.pw != "noir";
                        self.unlocked = !self.pw_err;
                        self.pw.clear();
                    }
                    if self.pw_err {
                        ui.label("Wrong password.");
                    }
                } else if kind == 1 {
                    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                        for h in self.hist.iter().rev().take(200) {
                            ui.label(h);
                        }
                    });
                    if ui.button("Lock").clicked() {
                        self.unlocked = false;
                    }
                } else {
                    egui::ComboBox::from_label("Search engine").selected_text(ENGINES[self.engine].0).show_ui(ui, |ui| {
                        for (i, e) in ENGINES.iter().enumerate() {
                            ui.selectable_value(&mut self.engine, i, e.0);
                        }
                    });
                    ui.label("Theme: black and white. AI features: none.");
                    if ui.button("Lock").clicked() {
                        self.unlocked = false;
                    }
                }
            });
            if !open {
                self.panel = 0;
            }
        }
    }
}
fn lum(c: Color32) -> f32 {
    (0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32) / 255.0
}

fn draw(ui: &mut egui::Ui, b: &Block, page_bg: Color32, nav: &mut Option<String>) {
    if b.mt > 0.0 {
        ui.add_space(b.mt.min(40.0));
    }
    match b.bg {
        Some(bg) => {
            egui::Frame::none().fill(bg).inner_margin(6.0).show(ui, |ui| {
                ui.set_width(ui.available_width());
                draw_inner(ui, b, bg, nav);
            });
        }
        None => draw_inner(ui, b, page_bg, nav),
    }
}

fn draw_inner(ui: &mut egui::Ui, b: &Block, bg: Color32, nav: &mut Option<String>) {
    let bl = lum(bg);
    let def = if bl > 0.5 { Color32::BLACK } else { Color32::WHITE };
    let pick = |r: &Run| match r.color {
        Some(c) if (lum(c) - bl).abs() > 0.25 => c,
        _ => def,
    };
    let has_link = b.runs.iter().any(|r| r.link.is_some());
    if b.center && !has_link {
        let mut job = egui::text::LayoutJob::default();
        job.halign = egui::Align::Center;
        job.wrap.max_width = ui.available_width();
        for r in &b.runs {
            job.append(&r.text, 0.0, egui::TextFormat { font_id: egui::FontId::proportional(r.size), color: pick(r), ..Default::default() });
        }
        ui.vertical_centered(|ui| {
            ui.label(job);
        });
    } else {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for r in &b.runs {
                let mut t = RichText::new(&r.text).size(r.size).color(pick(r));
                if r.bold {
                    t = t.strong();
                }
                match &r.link {
                    Some(u) => {
                        if ui.link(t).clicked() {
                            *nav = Some(u.clone());
                        }
                    }
                    None => {
                        ui.label(t);
                    }
                }
            }
        });
    }
}

fn fetch(url: &str) -> Result<String, String> {
    ureq::get(url)
        .set("User-Agent", "Noir/0.0.2 (beta)")
        .timeout(std::time::Duration::from_secs(15))
        .call()
        .map_err(|e| e.to_string())?
        .into_string()
        .map_err(|e| e.to_string())
}

fn link_hrefs(html: &str) -> Vec<String> {
    let low = html.to_ascii_lowercase();
    let mut v = Vec::new();
    let mut pos = 0;
    while pos < low.len() {
        let a = match low[pos..].find("<link") {
            Some(a) => pos + a,
            None => break,
        };
        let e = low[a..].find('>').map(|x| x + a).unwrap_or(low.len());
        let tag = &html[a..e];
        if tag.to_ascii_lowercase().contains("stylesheet") {
            if let Some(h) = attr(tag, "href") {
                v.push(h);
            }
        }
        pos = e + 1;
    }
    v
}

fn load_page(url: &str) -> Result<Page, String> {
    let html = fetch(url)?;
    let mut css = String::new();
    for h in link_hrefs(&html).into_iter().take(6) {
        if let Some(u) = resolve(url, &h) {
            if let Ok(t) = fetch(&u) {
                css.push_str(&t.chars().take(300_000).collect::<String>());
                css.push('\n');
            }
        }
    }
    Ok(parse(&html, url, &css))
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

fn to_url(input: &str, engine: usize) -> String {
    let s = input.trim();
    if s.starts_with("http://") || s.starts_with("https://") {
        s.to_string()
    } else if s.contains('.') && !s.contains(' ') {
        format!("https://{s}")
    } else {
        format!("{}{}", ENGINES[engine].1, enc(s))
    }
}

fn decode(s: &str) -> String {
    s.replace("&nbsp;", " ").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'").replace("&amp;", "&")
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let key = format!(" {name}=");
    let s = tag.find(&key)? + key.len();
    let rest = &tag[s..];
    let q = rest.chars().next()?;
    if q == '"' || q == '\'' {
        let body = &rest[1..];
        let e = body.find(q)?;
        Some(body[..e].to_string())
    } else {
        Some(rest.split(|c: char| c.is_whitespace()).next()?.to_string())
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
// ---------- CSS ----------

#[derive(Clone)]
struct Sel {
    tag: Option<String>,
    id: Option<String>,
    classes: Vec<String>,
    spec: u32,
}

struct Rule {
    sel: Sel,
    decls: Vec<(String, String)>,
    order: usize,
}

fn push_part(sel: &mut Sel, cur: &mut String, mode: char) {
    if cur.is_empty() {
        return;
    }
    match mode {
        '#' => {
            sel.id = Some(cur.clone());
            sel.spec += 100;
        }
        '.' => {
            sel.classes.push(cur.clone());
            sel.spec += 10;
        }
        _ => {
            if cur != "*" {
                sel.tag = Some(cur.to_lowercase());
                sel.spec += 1;
            }
        }
    }
    cur.clear();
}

fn parse_sel(c: &str) -> Option<Sel> {
    let mut sel = Sel { tag: None, id: None, classes: vec![], spec: 0 };
    let mut cur = String::new();
    let mut mode = 't';
    for ch in c.chars() {
        if ch == '.' || ch == '#' {
            push_part(&mut sel, &mut cur, mode);
            mode = ch;
        } else {
            cur.push(ch);
        }
    }
    push_part(&mut sel, &mut cur, mode);
    Some(sel)
}

fn parse_decls(b: &str) -> Vec<(String, String)> {
    b.split(';')
        .filter_map(|d| {
            let (k, v) = d.split_once(':')?;
            Some((k.trim().to_lowercase(), v.trim().to_string()))
        })
        .collect()
}

fn parse_css(css: &str) -> Vec<Rule> {
    let mut s = String::new();
    let mut rest = css;
    while let Some(a) = rest.find("/*") {
        s.push_str(&rest[..a]);
        match rest[a..].find("*/") {
            Some(b) => rest = &rest[a + b + 2..],
            None => rest = "",
        }
    }
    s.push_str(rest);
    let b = s.as_bytes();
    let mut rules: Vec<Rule> = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let start = i;
        while i < b.len() && b[i] != b'{' {
            i += 1;
        }
        if i >= b.len() {
            break;
        }
        let sel = s[start..i].trim().to_string();
        i += 1;
        let body_start = i;
        let mut depth = 1;
        while i < b.len() && depth > 0 {
            if b[i] == b'{' {
                depth += 1;
            } else if b[i] == b'}' {
                depth -= 1;
            }
            i += 1;
        }
        let body_end = if depth == 0 { i - 1 } else { i };
        if sel.starts_with('@') || body_end < body_start {
            continue;
        }
        let decls = parse_decls(&s[body_start..body_end]);
        for part in sel.split(',') {
            let last = part.split(|c: char| c.is_whitespace() || c == '>' || c == '+' || c == '~').filter(|x| !x.is_empty()).last();
            if let Some(l) = last {
                if l.contains(':') || l.contains('[') {
                    continue;
                }
                if let Some(sl) = parse_sel(l) {
                    let order = rules.len();
                    rules.push(Rule { sel: sl, decls: decls.clone(), order });
                }
            }
        }
    }
    rules
}

fn matches(s: &Sel, tag: &str, id: &str, classes: &[String]) -> bool {
    if let Some(t) = &s.tag {
        if t != tag {
            return false;
        }
    }
    if let Some(i) = &s.id {
        if i != id {
            return false;
        }
    }
    s.classes.iter().all(|c| classes.contains(c))
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

fn apply(s: &mut Style, k: &str, v: &str, parent_size: f32) {
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
            if let Some(n) = parse_len(v, parent_size) {
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
                s.hidden = true;
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
            }
        }
        "margin" => {
            if let Some(n) = v.split_whitespace().next().and_then(|t| parse_len(t, 15.0)) {
                s.mt = n.clamp(0.0, 60.0);
                s.mb = s.mt;
            }
        }
        _ => {}
    }
}

fn ua(s: &mut Style, tag: &str) {
    match tag {
        "h1" => {
            s.size = 30.0;
            s.bold = true;
            s.mt = 12.0;
            s.mb = 8.0;
        }
        "h2" => {
            s.size = 24.0;
            s.bold = true;
            s.mt = 10.0;
            s.mb = 6.0;
        }
        "h3" => {
            s.size = 19.0;
            s.bold = true;
            s.mt = 8.0;
            s.mb = 4.0;
        }
        "h4" | "h5" | "h6" => {
            s.size = 16.0;
            s.bold = true;
            s.mt = 6.0;
            s.mb = 4.0;
        }
        "b" | "strong" | "th" => s.bold = true,
        "center" => s.center = true,
        "p" => {
            s.mt = 8.0;
            s.mb = 8.0;
        }
        "ul" | "ol" | "blockquote" => {
            s.mt = 6.0;
            s.mb = 6.0;
        }
        "li" => s.mt = 2.0,
        "small" => s.size *= 0.85,
        "big" => s.size *= 1.2,
        "head" => s.hidden = true,
        _ => {}
    }
}

fn compute(parent: &Style, tag: &str, id: &str, classes: &[String], inline: Option<String>, rules: &[Rule], href: Option<String>) -> Style {
    let mut s = parent.clone();
    s.bg = None;
    s.mt = 0.0;
    s.mb = 0.0;
    ua(&mut s, tag);
    if let Some(h) = href {
        s.link = Some(h);
        s.color = Some(Color32::from_rgb(130, 170, 255));
    }
    let mut m: Vec<&Rule> = rules.iter().filter(|r| matches(&r.sel, tag, id, classes)).collect();
    m.sort_by_key(|r| (r.sel.spec, r.order));
    for r in m {
        for (k, v) in &r.decls {
            apply(&mut s, k, v, parent.size);
        }
    }
    if let Some(st) = inline {
        for (k, v) in parse_decls(&st) {
            apply(&mut s, &k, &v, parent.size);
        }
    }
    s
}
// ---------- HTML ----------

struct P {
    out: Vec<Block>,
    runs: Vec<Run>,
    text: String,
    pend: f32,
}

impl P {
    fn end_run(&mut self, st: &Style) {
        if self.text.is_empty() {
            return;
        }
        let t = decode(&self.text);
        self.text.clear();
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
}

fn parse(html: &str, base: &str, ext: &str) -> Page {
    let c: Vec<char> = html.chars().collect();
    let mut css = String::from(ext);
    let mut rules = parse_css(&css);
    let mut p = P { out: Vec::new(), runs: Vec::new(), text: String::new(), pend: 0.0 };
    let mut stack: Vec<Frame> = Vec::new();
    let mut title = String::new();
    let mut page_bg: Option<Color32> = None;
    let mut skip = false;
    let mut in_style = false;
    let mut in_title = false;
    let mut i = 0;
    while i < c.len() {
        if c[i] == '<' {
            let mut j = i + 1;
            while j < c.len() && c[j] != '>' {
                j += 1;
            }
            let tag: String = c[i + 1..j].iter().collect();
            i = j + 1;
            let low = tag.trim().to_lowercase();
            let closing = low.starts_with('/');
            let name: String = low.trim_start_matches('/').chars().take_while(|ch| ch.is_ascii_alphanumeric()).collect();
            if skip && !(closing && (name == "script" || name == "style")) {
                continue;
            }
            if name.is_empty() {
                continue;
            }
            if name == "script" || name == "style" {
                if closing {
                    skip = false;
                    if in_style {
                        in_style = false;
                        rules = parse_css(&css);
                    }
                } else {
                    skip = true;
                    in_style = name == "style";
                    css.push('\n');
                }
                continue;
            }
            if name == "title" {
                in_title = !closing;
                continue;
            }
            let cur = stack.last().map(|f| f.style.clone()).unwrap_or_else(Style::base);
            p.end_run(&cur);
            let block = BLOCKS.contains(&name.as_str());
            if block {
                let bg = stack.iter().rev().find_map(|f| f.style.bg);
                p.flush(&cur, bg);
            }
            if closing {
                if let Some(pos) = stack.iter().rposition(|f| f.tag == name) {
                    if block {
                        p.pend = p.pend.max(stack[pos].style.mb);
                    }
                    stack.truncate(pos);
                }
            } else if !VOID.contains(&name.as_str()) {
                let id = attr(&tag, "id").unwrap_or_default();
                let classes: Vec<String> = attr(&tag, "class").unwrap_or_default().split_whitespace().map(|s| s.to_string()).collect();
                let href = if name == "a" { attr(&tag, "href").and_then(|h| resolve(base, &h)) } else { None };
                let mut s = compute(&cur, &name, &id, &classes, attr(&tag, "style"), &rules, href);
                if name == "html" || name == "body" {
                    if s.bg.is_some() {
                        page_bg = s.bg;
                    }
                    s.bg = None;
                }
                if block {
                    p.pend = p.pend.max(s.mt);
                }
                let hidden = s.hidden;
                stack.push(Frame { tag: name.clone(), style: s });
                if name == "li" && !hidden {
                    p.text.push_str("• ");
                }
            }
        } else {
            let ch = c[i];
            i += 1;
            if skip {
                if in_style {
                    css.push(ch);
                }
                continue;
            }
            let ws = ch.is_whitespace();
            if in_title {
                if !ws || (!title.is_empty() && !title.ends_with(' ')) {
                    title.push(if ws { ' ' } else { ch });
                }
                continue;
            }
            if stack.last().map(|f| f.style.hidden).unwrap_or(false) {
                continue;
            }
            if ws {
                if !p.text.is_empty() {
                    if !p.text.ends_with(' ') {
                        p.text.push(' ');
                    }
                } else if p.runs.last().map(|r| !r.text.ends_with(' ')).unwrap_or(false) {
                    p.text.push(' ');
                }
            } else {
                p.text.push(ch);
            }
        }
    }
    let cur = stack.last().map(|f| f.style.clone()).unwrap_or_else(Style::base);
    p.flush(&cur, None);
    Page { title: decode(title.trim()), bg: page_bg, blocks: p.out }
}

fn visuals() -> egui::Visuals {
    let mut v = egui::Visuals::dark();
    v.panel_fill = Color32::BLACK;
    v.window_fill = Color32::BLACK;
    v.extreme_bg_color = Color32::BLACK;
    v.faint_bg_color = Color32::from_gray(14);
    v.hyperlink_color = Color32::WHITE;
    v
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1100.0, 720.0]).with_title("Noir 0.0.2 beta"),
        ..Default::default()
    };
    eframe::run_native(
        "Noir",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(visuals());
            Ok(Box::new(Noir::new()))
        }),
    )
}