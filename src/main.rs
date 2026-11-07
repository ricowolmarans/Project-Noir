use eframe::egui::{self, Color32, RichText};
use std::sync::mpsc::{channel, Receiver, Sender};

const ENGINES: [(&str, &str); 2] = [
    ("Google", "https://www.google.com/search?q="),
    ("DuckDuckGo Lite", "https://lite.duckduckgo.com/lite/?q="),
];

#[derive(Clone)]
enum Kind {
    Head(f32),
    Text,
    Link(String),
}

#[derive(Clone)]
struct Block {
    kind: Kind,
    text: String,
}

type Msg = (u64, String, Result<(String, Vec<Block>), String>);

struct Tab {
    id: u64,
    title: String,
    url: String,
    blocks: Vec<Block>,
    asleep: bool,
    loading: bool,
    err: Option<String>,
}

impl Tab {
    fn new(id: u64) -> Self {
        Tab { id, title: "New tab".into(), url: String::new(), blocks: vec![], asleep: false, loading: false, err: None }
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
            let r = fetch(&url).map(|h| parse(&h, &url));
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
                    Ok((title, blocks)) => {
                        t.title = if title.is_empty() { url.clone() } else { title };
                        t.blocks = blocks;
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
        egui::CentralPanel::default().show(ctx, |ui| {
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
                    match &b.kind {
                        Kind::Head(s) => {
                            ui.add_space(6.0);
                            ui.label(RichText::new(&b.text).size(*s).strong());
                        }
                        Kind::Text => {
                            ui.label(&b.text);
                        }
                        Kind::Link(u) => {
                            if ui.link(&b.text).clicked() {
                                nav = Some(u.clone());
                            }
                        }
                    }
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

fn fetch(url: &str) -> Result<String, String> {
    ureq::get(url)
        .set("User-Agent", "Noir/0.0.1 (beta)")
        .timeout(std::time::Duration::from_secs(15))
        .call()
        .map_err(|e| e.to_string())?
        .into_string()
        .map_err(|e| e.to_string())
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

fn flush(out: &mut Vec<Block>, text: &mut String, kind: &Kind) {
    let t = text.trim();
    if !t.is_empty() && out.len() < 3000 {
        out.push(Block { kind: kind.clone(), text: decode(t) });
    }
    text.clear();
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let key = format!("{name}=");
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
    if h.is_empty() || h.starts_with('#') || h.starts_with("javascript:") || h.starts_with("mailto:") {
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

fn parse(html: &str, base: &str) -> (String, Vec<Block>) {
    let c: Vec<char> = html.chars().collect();
    let mut out = Vec::new();
    let mut title = String::new();
    let mut text = String::new();
    let mut kind = Kind::Text;
    let mut skip = false;
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
            match name.as_str() {
                "script" | "style" => skip = !closing,
                "title" => in_title = !closing,
                "h1" | "h2" | "h3" => {
                    flush(&mut out, &mut text, &kind);
                    kind = if closing {
                        Kind::Text
                    } else {
                        Kind::Head(match name.as_str() {
                            "h1" => 30.0,
                            "h2" => 24.0,
                            _ => 19.0,
                        })
                    };
                }
                "a" => {
                    flush(&mut out, &mut text, &kind);
                    kind = if closing {
                        Kind::Text
                    } else {
                        match attr(&tag, "href").and_then(|h| resolve(base, &h)) {
                            Some(u) => Kind::Link(u),
                            None => Kind::Text,
                        }
                    };
                }
                "p" | "div" | "br" | "li" | "tr" | "section" | "article" | "ul" | "ol" | "table" | "header" | "footer" | "nav" => flush(&mut out, &mut text, &kind),
                _ => {}
            }
        } else {
            let ch = c[i];
            i += 1;
            if skip {
                continue;
            }
            let dst = if in_title { &mut title } else { &mut text };
            if ch.is_whitespace() {
                if !dst.is_empty() && !dst.ends_with(' ') {
                    dst.push(' ');
                }
            } else {
                dst.push(ch);
            }
        }
    }
    flush(&mut out, &mut text, &kind);
    (decode(title.trim()), out)
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
        viewport: egui::ViewportBuilder::default().with_inner_size([1100.0, 720.0]).with_title("Noir 0.0.1 beta"),
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
