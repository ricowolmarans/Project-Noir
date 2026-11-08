mod page;

use eframe::egui::{self, Color32, RichText};
use page::{load_page, to_url, Block, Page, Run, ENGINES};
use std::sync::mpsc::{channel, Receiver, Sender};

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
    back: Vec<String>,
    fwd: Vec<String>,
}

impl Tab {
    fn new(id: u64) -> Self {
        Tab { id, title: "New tab".into(), url: String::new(), blocks: vec![], bg: None, asleep: false, loading: false, err: None, back: vec![], fwd: vec![] }
    }
}

struct Noir {
    tabs: Vec<Tab>,
    cur: usize,
    next_id: u64,
    addr: String,
    start_q: String,
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
        Noir {
            tabs: vec![Tab::new(1)],
            cur: 0,
            next_id: 2,
            addr: String::new(),
            start_q: String::new(),
            game: false,
            hist: vec![],
            unlocked: false,
            pw: String::new(),
            pw_err: false,
            panel: 0,
            engine: 0,
            tx,
            rx,
        }
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

    fn navigate(&mut self, url: String, ctx: &egui::Context) {
        let c = self.cur;
        let old = self.tabs[c].url.clone();
        if !old.is_empty() && old != url {
            self.tabs[c].back.push(old);
        }
        self.tabs[c].fwd.clear();
        self.load(c, url, ctx);
    }

    fn go(&mut self, input: &str, ctx: &egui::Context) {
        let url = to_url(input, self.engine);
        self.navigate(url, ctx);
    }

    fn back(&mut self, ctx: &egui::Context) {
        let c = self.cur;
        if let Some(u) = self.tabs[c].back.pop() {
            let now = self.tabs[c].url.clone();
            if !now.is_empty() {
                self.tabs[c].fwd.push(now);
            }
            self.load(c, u, ctx);
        }
    }

    fn forward(&mut self, ctx: &egui::Context) {
        let c = self.cur;
        if let Some(u) = self.tabs[c].fwd.pop() {
            let now = self.tabs[c].url.clone();
            if !now.is_empty() {
                self.tabs[c].back.push(now);
            }
            self.load(c, u, ctx);
        }
    }

    fn reload(&mut self, ctx: &egui::Context) {
        let c = self.cur;
        let u = self.tabs[c].url.clone();
        if !u.is_empty() {
            self.load(c, u, ctx);
        }
    }

    fn home(&mut self) {
        let c = self.cur;
        let t = &mut self.tabs[c];
        t.url.clear();
        t.blocks.clear();
        t.title = "New tab".into();
        t.bg = None;
        t.err = None;
        t.loading = false;
        self.addr.clear();
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

fn short(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
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

        // ---------- sidebar ----------
        let mut sw = None;
        let mut cl = None;
        let mut add = false;
        egui::SidePanel::left("tabs")
            .exact_width(230.0)
            .frame(egui::Frame::none().fill(Color32::from_gray(10)).inner_margin(12.0))
            .show(ctx, |ui| {
                ui.label(RichText::new("Noir").size(28.0).strong());
                ui.add_space(10.0);
                if ui.add_sized([ui.available_width(), 32.0], egui::Button::new("+  New tab")).clicked() {
                    add = true;
                }
                ui.add_space(8.0);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (i, t) in self.tabs.iter().enumerate() {
                        ui.horizontal(|ui| {
                            let label = format!("{}{}", if t.asleep { "zzz " } else { "" }, short(&t.title, 18));
                            let w = (ui.available_width() - 34.0).max(40.0);
                            if ui.add_sized([w, 28.0], egui::SelectableLabel::new(i == self.cur, label)).clicked() {
                                sw = Some(i);
                            }
                            if ui.add_sized([24.0, 24.0], egui::Button::new("x").frame(false)).clicked() {
                                cl = Some(i);
                            }
                        });
                    }
                });
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

        // ---------- toolbar ----------
        let mut go_to: Option<String> = None;
        let mut act = 0u8;
        let can_b = !self.tabs[self.cur].back.is_empty();
        let can_f = !self.tabs[self.cur].fwd.is_empty();
        let loading = self.tabs[self.cur].loading;
        egui::TopBottomPanel::top("bar")
            .frame(egui::Frame::none().fill(Color32::BLACK).inner_margin(egui::Margin::symmetric(12.0, 10.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.add_enabled(can_b, egui::Button::new("<")).clicked() {
                        act = 1;
                    }
                    if ui.add_enabled(can_f, egui::Button::new(">")).clicked() {
                        act = 2;
                    }
                    if ui.button("Reload").clicked() {
                        act = 3;
                    }
                    if ui.button("Home").clicked() {
                        act = 4;
                    }
                    let w = (ui.available_width() - 200.0).max(100.0);
                    let hint = format!("Search {} or type a URL", ENGINES[self.engine].0);
                    let r = ui.add_sized([w, 30.0], egui::TextEdit::singleline(&mut self.addr).hint_text(hint));
                    if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        go_to = Some(self.addr.clone());
                    }
                    if loading {
                        ui.spinner();
                    }
                    if ui.button("History").clicked() {
                        self.panel = 1;
                    }
                    if ui.button("Settings").clicked() {
                        self.panel = 2;
                    }
                });
            });
        match act {
            1 => self.back(ctx),
            2 => self.forward(ctx),
            3 => self.reload(ctx),
            4 => self.home(),
            _ => {}
        }

        // ---------- status bar ----------
        let asleep = self.tabs.iter().filter(|t| t.asleep).count();
        let mut toggled = false;
        egui::TopBottomPanel::bottom("status")
            .frame(egui::Frame::none().fill(Color32::from_gray(10)).inner_margin(egui::Margin::symmetric(12.0, 6.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut self.game, "Gaming mode (sleeps background tabs)").changed() {
                        toggled = true;
                    }
                    ui.label(RichText::new(format!("{} asleep", asleep)).color(Color32::GRAY));
                    ui.label(RichText::new(format!("Search: {}", ENGINES[self.engine].0)).color(Color32::GRAY));
                });
            });
        if toggled {
            self.sleep_others();
        }

        // ---------- page area ----------
        let mut nav: Option<String> = None;
        let mut start_go: Option<String> = None;
        let panel_bg = self.tabs[self.cur].bg.unwrap_or(Color32::BLACK);
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(panel_bg).inner_margin(16.0))
            .show(ctx, |ui| {
                let empty = self.tabs[self.cur].url.is_empty();
                if empty {
                    ui.vertical_centered(|ui| {
                        ui.add_space(ui.available_height() * 0.28);
                        ui.label(RichText::new("Noir").size(84.0).strong());
                        ui.label(RichText::new("Private. Black and white. No AI.").color(Color32::GRAY));
                        ui.add_space(20.0);
                        let hint = format!("Search {}", ENGINES[self.engine].0);
                        let r = ui.add_sized([480.0, 38.0], egui::TextEdit::singleline(&mut self.start_q).hint_text(hint));
                        if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            start_go = Some(self.start_q.clone());
                        }
                    });
                    return;
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.set_max_width(980.0);
                    let t = &self.tabs[self.cur];
                    if t.loading && t.blocks.is_empty() {
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
        if let Some(s) = start_go {
            self.start_q.clear();
            if !s.trim().is_empty() {
                self.go(s.trim(), ctx);
            }
        }
        if let Some(s) = go_to {
            if !s.trim().is_empty() {
                self.go(s.trim(), ctx);
            }
        }
        if let Some(u) = nav {
            self.navigate(u, ctx);
        }

        // ---------- History and Settings ----------
        if self.panel != 0 {
            let kind = self.panel;
            let mut open = true;
            egui::Window::new(if kind == 1 { "History" } else { "Settings" })
                .open(&mut open)
                .show(ctx, |ui| {
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
                        egui::ComboBox::from_label("Search engine")
                            .selected_text(ENGINES[self.engine].0)
                            .show_ui(ui, |ui| {
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
            job.append(
                &r.text,
                0.0,
                egui::TextFormat { font_id: egui::FontId::proportional(r.size), color: pick(r), ..Default::default() },
            );
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

fn visuals() -> egui::Visuals {
    let mut v = egui::Visuals::dark();
    v.panel_fill = Color32::BLACK;
    v.window_fill = Color32::from_gray(8);
    v.extreme_bg_color = Color32::from_gray(18);
    v.faint_bg_color = Color32::from_gray(14);
    v.hyperlink_color = Color32::WHITE;
    v.selection.bg_fill = Color32::WHITE;
    v.selection.stroke = egui::Stroke::new(1.0, Color32::BLACK);
    let r = egui::Rounding::same(14.0);
    v.widgets.noninteractive.rounding = r;
    v.widgets.inactive.rounding = r;
    v.widgets.hovered.rounding = r;
    v.widgets.active.rounding = r;
    v.widgets.open.rounding = r;
    v.widgets.inactive.bg_fill = Color32::from_gray(22);
    v.widgets.inactive.weak_bg_fill = Color32::from_gray(22);
    v.widgets.hovered.bg_fill = Color32::from_gray(45);
    v.widgets.hovered.weak_bg_fill = Color32::from_gray(45);
    v
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 720.0])
            .with_title("Noir 0.1.0 beta"),
        ..Default::default()
    };
    eframe::run_native(
        "Noir",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(visuals());
            cc.egui_ctx.style_mut(|s| {
                s.spacing.item_spacing = egui::vec2(8.0, 8.0);
                s.spacing.button_padding = egui::vec2(12.0, 6.0);
            });
            Ok(Box::new(Noir::new()))
        }),
    )
}