#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
use eframe::egui;
use egui::{Context, Layout};
use egui_extras::{Column, TableBuilder};
use log::{error, info, warn};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;

#[derive(Default, Debug, Clone)]
struct ParsedFile {
    file_name: String,
    // address -> data
    address_to_data: BTreeMap<u64, String>,
}

#[derive(Default)]
struct AppState {
    files: Vec<ParsedFile>,
    // Cached intersection of all addresses across selected files
    intersect_addresses: Vec<u64>,
    // UI
    show_stats: bool,
    selected_row: Option<usize>,
    display_base: DisplayBase,
    stats_metric: StatsMetric,
    chart_alpha: f32,
}

impl AppState {
    fn new() -> Self {
        Self { display_base: DisplayBase::Hex, stats_metric: StatsMetric::Count, chart_alpha: 0.8, ..Default::default() }
    }

    fn recalc_intersection(&mut self) {
        let mut iter = self.files.iter();
        let Some(first) = iter.next() else {
            self.intersect_addresses.clear();
            return;
        };
        let mut set: BTreeSet<u64> = first.address_to_data.keys().copied().collect();
        for pf in iter {
            let other: BTreeSet<u64> = pf.address_to_data.keys().copied().collect();
            set = set.intersection(&other).copied().collect();
        }
        self.intersect_addresses = set.into_iter().collect();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DisplayBase { Hex, Bin, Dec }

impl Default for DisplayBase {
    fn default() -> Self { DisplayBase::Hex }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StatsMetric { Percent, Count }

impl Default for StatsMetric {
    fn default() -> Self { StatsMetric::Count }
}

// Generate a convex polygon approximating a pie slice from start_angle to end_angle
fn pie_slice(center: egui::Pos2, radius: f32, start_angle: f32, end_angle: f32) -> Vec<egui::Pos2> {
    let mut points = Vec::new();
    points.push(center);
    let steps = 48usize;
    let mut a = start_angle;
    let step = ((end_angle - start_angle) / steps as f32).abs().max(0.01);
    while a < end_angle {
        let x = center.x + radius * a.cos();
        let y = center.y + radius * a.sin();
        points.push(egui::pos2(x, y));
        a += step;
    }
    // ensure final point at end_angle
    let x = center.x + radius * end_angle.cos();
    let y = center.y + radius * end_angle.sin();
    points.push(egui::pos2(x, y));
    points
}

// Format address as hex with 0x prefix, pad to even length and at least 2 digits
fn format_addr(addr: u64) -> String {
    let mut s = format!("{:x}", addr); // lowercase, no prefix
    if s.len() < 2 { s = format!("{:02x}", addr); }
    if s.len() % 2 != 0 { s = format!("0{}", s); }
    format!("0x{}", s)
}

fn format_data_with_base(raw: &str, base: DisplayBase) -> String {
    let cleaned = raw.trim().trim_end_matches('h');
    match base {
        DisplayBase::Hex => {
            // Show as 0xXX (at least 2 hex digits, even length)
            if let Ok(v) = u64::from_str_radix(cleaned.trim_start_matches("0x"), 16) {
                return format_hex_prefixed_min2_even(v);
            } else if let Ok(vd) = cleaned.parse::<u64>() {
                return format_hex_prefixed_min2_even(vd);
            }
            cleaned.to_string()
        }
        DisplayBase::Dec => {
            // try hex then dec
            if let Ok(v) = u64::from_str_radix(cleaned.trim_start_matches("0x"), 16) { v.to_string() }
            else if let Ok(vd) = cleaned.parse::<u64>() { vd.to_string() }
            else { cleaned.to_string() }
        }
        DisplayBase::Bin => {
            if let Ok(v) = u64::from_str_radix(cleaned.trim_start_matches("0x"), 16) { format!("{:08b}", v & 0xFF) }
            else if let Ok(vd) = cleaned.parse::<u64>() { format!("{:08b}", vd & 0xFF) }
            else { cleaned.to_string() }
        }
    }
}

fn format_hex_prefixed_min2_even(v: u64) -> String {
    let mut s = format!("{:x}", v);
    if s.len() < 2 { s = format!("{:02x}", v); }
    if s.len() % 2 != 0 { s = format!("0{}", s); }
    format!("0x{}", s)
}

// Generate N distinct colors by evenly spacing hues on the HSV circle
fn generate_palette(count: usize) -> Vec<egui::Color32> {
    if count == 0 { return Vec::new(); }
    (0..count)
        .map(|i| hsv_to_rgb((i as f32) / (count as f32), 0.65, 0.92))
        .collect()
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> egui::Color32 {
    // h in [0,1), s in [0,1], v in [0,1]
    let h6 = (h * 6.0).fract();
    let i = (h * 6.0).floor() as i32;
    let f = h6;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    let (r, g, b) = match i.rem_euclid(6) {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    egui::Color32::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

#[cfg(target_os = "windows")]
fn load_app_icon() -> Option<egui::IconData> {
    // Embed S.png at compile time so runtime无需依赖外部文件
    const BYTES: &[u8] = include_bytes!("../../S.png");
    let image = image::load_from_memory(BYTES).ok()?.to_rgba8();
    let (w, h) = image.dimensions();
    Some(egui::IconData { rgba: image.into_raw(), width: w, height: h })
}

#[cfg(not(target_os = "windows"))]
fn load_app_icon() -> Option<egui::IconData> { None }

// removed: bold font installation
fn parse_txt_file(path: &str) -> anyhow::Result<ParsedFile> {
    let content = fs::read_to_string(path)?;
    let mut address_to_data: BTreeMap<u64, String> = BTreeMap::new();
    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.eq_ignore_ascii_case("END") {
            break;
        }
        // split by tab
        let parts: Vec<&str> = trimmed.split('\t').collect();
        if parts.len() < 3 {
            warn!("跳过第{idx}行：列数不足: {trimmed}");
            continue;
        }
        // 样例：0001\t0\t02\t02\t02h\t7E\t126\t
        // 约定：第3列为十六进制地址，第3列为十六进制数据（需求文字重复处，这里按常理采用第3列=地址，第5列或第6列不使用）。
        // 这里我们采用：第3列为地址，第4列为数据（若没有第4列则跳过）。
        // 如需调整，请告知具体列位。
        let addr_str = parts[2].trim();
        let data_str = parts.get(5).map(|s| s.trim()).unwrap_or("");
        if addr_str.is_empty() || data_str.is_empty() {
            warn!("跳过第{idx}行：地址或数据为空: {trimmed}");
            continue;
        }

        // 允许 0x 前缀或纯十六进制，不含 h 尾缀；若存在 h 尾缀则去掉
        let addr_clean = addr_str.trim_end_matches('h').trim_start_matches("0x");
        match u64::from_str_radix(addr_clean, 16) {
            Ok(address) => {
                address_to_data.insert(address, data_str.to_string());
            }
            Err(e) => {
                warn!("解析地址失败 第{idx}行: {addr_str}, 错误: {e}");
            }
        }
    }
    let file_name = std::path::Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(path)
        .to_string();
    Ok(ParsedFile { file_name, address_to_data })
}

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let mut native_options = eframe::NativeOptions::default();
    #[cfg(target_os = "windows")]
    {
        if let Some(icon) = load_app_icon() {
            native_options.viewport.icon = Some(icon.into());
        }
    }
    eframe::run_native(
        "SuffixCode Viewer V0.1",
        native_options,
        Box::new(|_cc| Box::new(AppState::new())),
    )
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
                    if ui.button("Add").clicked() {
                        let files = rfd::FileDialog::new()
                            .add_filter("Text", &["txt"]).pick_files();
                        if let Some(paths) = files {
                            for path in paths {
                                match parse_txt_file(path.to_string_lossy().as_ref()) {
                                    Ok(pf) => {
                                        info!("Parsed: {} ({} rows)", pf.file_name, pf.address_to_data.len());
                                        self.files.push(pf);
                                    }
                                    Err(e) => {
                                        error!("Parse failed: {:?}", e);
                                    }
                                }
                            }
                            self.recalc_intersection();
                        }
                    }

                    if ui.button("Clear").clicked() {
                        self.files.clear();
                        self.intersect_addresses.clear();
                    }

                    if ui.button("Stats").clicked() {
                        self.show_stats = true;
                    }

                    if ui.button("Export").clicked() {
                        // Export CSV: first column is address, then one column per file's data
                        if !self.intersect_addresses.is_empty() && !self.files.is_empty() {
                            let mut csv = String::new();
                            // header
                            csv.push_str("address");
                            for pf in &self.files {
                                csv.push(',');
                                csv.push_str(&pf.file_name);
                            }
                            csv.push('\n');
                            // rows
                            for addr in &self.intersect_addresses {
                                csv.push_str(&format_addr(*addr));
                                for pf in &self.files {
                                    let raw = pf.address_to_data.get(addr).cloned().unwrap_or_default();
                                    let val = format_data_with_base(&raw, self.display_base);
                                    csv.push(',');
                                    csv.push_str(&val);
                                }
                                csv.push('\n');
                            }
                            if let Some(path) = rfd::FileDialog::new().set_file_name("export.csv").save_file() {
                                if let Err(e) = fs::write(&path, csv) {
                                    error!("Export failed: {:?}", e);
                                } else {
                                    info!("Exported: {}", path.to_string_lossy());
                                }
                            }
                        } else {
                            warn!("No data to export");
                        }
                    }
                });

                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    let label = match self.display_base { DisplayBase::Hex => "HEX", DisplayBase::Bin => "BIN", DisplayBase::Dec => "DEC" };
                    if ui.button(label).clicked() {
                        self.display_base = match self.display_base {
                            DisplayBase::Hex => DisplayBase::Bin,
                            DisplayBase::Bin => DisplayBase::Dec,
                            DisplayBase::Dec => DisplayBase::Hex,
                        };
                    }
                    ui.label("Base:");
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.files.is_empty() {
                ui.label("No Suffix Code files. Table is empty.");
                // Empty table placeholder
                TableBuilder::new(ui)
                    .striped(true)
                    .column(Column::initial(120.0).resizable(true))
                    .header(20.0, |mut header| {
                        header.col(|ui| { ui.label("Address"); });
                    })
                    .body(|_body| {});
                return;
            }

            // 构建列：1列地址 + N列数据，并支持水平滚动
            egui::ScrollArea::horizontal().show(ui, |ui| {
                let mut table = TableBuilder::new(ui).striped(true);
                table = table.column(Column::initial(140.0).resizable(true));
                for _ in &self.files { table = table.column(Column::initial(120.0).resizable(true)); }

                table
                    .header(24.0, |mut header| {
                        header.col(|ui| { ui.label("Address"); });
                        for pf in &self.files {
                            header.col(|ui| { ui.label(&pf.file_name); });
                        }
                    })
                    .body(|mut body| {
                        let mut row_idx: usize = 0;
                        for addr in &self.intersect_addresses {
                            body.row(22.0, |mut row| {
                                // Address column (click to select row)
                                row.col(|ui| {
                                    let is_selected = self.selected_row == Some(row_idx);
                                    let resp = ui.add(egui::SelectableLabel::new(is_selected, format_addr(*addr)));
                                    if resp.clicked() {
                                        self.selected_row = Some(row_idx);
                                    }
                                });
                                // Data columns
                                for pf in &self.files {
                                    row.col(|ui| {
                                        let raw = pf.address_to_data.get(addr).cloned().unwrap_or_default();
                                        ui.monospace(format_data_with_base(&raw, self.display_base));
                                    });
                                }
                            });
                            row_idx += 1;
                        }
                    });
            });

            // Note: multi-cell selection and copy features were removed per request.
        });

        // Handle drag-and-drop files (acts like Add)
        // Accepts .txt file paths; other cases are ignored with a warning
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if !dropped.is_empty() {
            let mut added_any = false;
            for f in dropped {
                if let Some(path) = f.path {
                    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                        if ext.eq_ignore_ascii_case("txt") {
                            match parse_txt_file(path.to_string_lossy().as_ref()) {
                                Ok(pf) => {
                                    info!("Parsed (drop): {} ({} rows)", pf.file_name, pf.address_to_data.len());
                                    self.files.push(pf);
                                    added_any = true;
                                }
                                Err(e) => {
                                    error!("Parse failed (drop): {:?}", e);
                                }
                            }
                        } else {
                            warn!("Ignored dropped file (not .txt): {}", path.to_string_lossy());
                        }
                    }
                } else {
                    warn!("Dropped data without a path is not supported");
                }
            }
            if added_any { self.recalc_intersection(); }
        }

        if self.show_stats {
            let main_rect = ctx.input(|i| i.screen_rect());
            egui::Window::new("Statistics")
                .constrain_to(main_rect)
                .max_size(main_rect.size())
                .open(&mut self.show_stats)
                .show(ctx, |ui| {
                ui.label(format!("Files: {}", self.files.len()));
                let total_rows: usize = self.files.iter().map(|f| f.address_to_data.len()).sum();
                ui.label(format!("Total rows: {}", total_rows));
                ui.label(format!("Total addresses: {}", self.intersect_addresses.len()));

                // Toggle Percent/Count and set color alpha & font scale
                ui.horizontal(|ui| {
                    let label = match self.stats_metric { StatsMetric::Percent => "Bar: Percent", StatsMetric::Count => "Bar: Count" };
                    if ui.button(label).clicked() {
                        self.stats_metric = match self.stats_metric { StatsMetric::Percent => StatsMetric::Count, StatsMetric::Count => StatsMetric::Percent };
                    }
                    ui.add(egui::Slider::new(&mut self.chart_alpha, 0.1..=1.0).text("Alpha").clamp_to_range(true));
                });

                if let Some(selected_row) = self.selected_row {
                    if let Some(&addr) = self.intersect_addresses.get(selected_row) {
                        ui.separator();
                        ui.label(format!("Selected Address: {}", format_addr(addr)));

                        // Group by data value: value_string -> Vec<file_name>
                        use std::collections::BTreeMap;
                        let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
                        for pf in &self.files {
                            let raw = pf.address_to_data.get(&addr).cloned().unwrap_or_default();
                            let shown = format_data_with_base(&raw, self.display_base);
                            groups.entry(shown).or_default().push(pf.file_name.clone());
                        }
                        let total = self.files.len() as f32;
                        let mut group_entries: Vec<(String, usize)> = groups.iter().map(|(k, v)| (k.clone(), v.len())).collect();
                        // Sort by count desc then key asc for stable display
                        group_entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

                        // Draw charts side-by-side (bars by count, pie by percentage)
                        ui.horizontal(|ui| {
                            // Bar chart
                            let desired = egui::vec2(360.0, 220.0);
                            let (rect, _resp) = ui.allocate_exact_size(desired, egui::Sense::hover());
                            let mut shapes = Vec::new();
                            let max_count = group_entries.iter().map(|(_, c)| *c as f32).fold(0.0, f32::max).max(1.0);
                            let bar_gap = 6.0;
                            let bar_count = group_entries.len() as f32;
                            let bar_width = ((rect.width() - bar_gap * (bar_count + 1.0)) / bar_count).max(2.0);
                            let mut colors = generate_palette(group_entries.len());
                            // apply alpha to colors
                            let a = (255.0 * self.chart_alpha) as u8;
                            for c in &mut colors { *c = egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a); }
                            for (idx, (value_label, count)) in group_entries.iter().enumerate() {
                                let h = (*count as f32 / max_count) * (rect.height() - 28.0);
                                let left = rect.left() + bar_gap + (bar_width + bar_gap) * idx as f32;
                                let r = egui::Rect::from_min_size(
                                    egui::pos2(left, rect.bottom() - h - 4.0),
                                    egui::vec2(bar_width, h),
                                );
                                let color = colors[idx % colors.len()];
                                shapes.push(egui::Shape::rect_filled(r, 2.0, color));
                                // label under bar (black text)
                                let galley = ui.painter().layout(
                                    value_label.clone(),
                                    egui::FontId::monospace(10.0),
                                    egui::Color32::BLACK,
                                    rect.width(),
                                );
                                let text_pos = egui::pos2(left, rect.bottom() - 14.0);
                                shapes.push(egui::Shape::galley(text_pos, galley, egui::Color32::BLACK));

                                // percentage/count above bar (black text)
                                let pct = (*count as f32 / total) * 100.0;
                                let label_text = match self.stats_metric { StatsMetric::Percent => format!("{:.1}%", pct), StatsMetric::Count => format!("{}", count) };
                                let galley_pct = ui.painter().layout(
                                    label_text,
                                    egui::FontId::monospace(10.0),
                                    egui::Color32::BLACK,
                                    rect.width(),
                                );
                                let pct_pos = egui::pos2(left, (r.top() - 12.0).max(rect.top() + 2.0));
                                shapes.push(egui::Shape::galley(pct_pos, galley_pct, egui::Color32::BLACK));
                            }
                            ui.painter().extend(shapes);

                            // Pie chart
                            let desired2 = egui::vec2(220.0, 220.0);
                            let (rect2, _resp2) = ui.allocate_exact_size(desired2, egui::Sense::hover());
                            let center = rect2.center();
                            let radius = rect2.size().min_elem() * 0.45;
                            let mut start_angle: f32 = 0.0;
                            let mut shapes2: Vec<egui::Shape> = Vec::new();
                            if group_entries.len() == 1 {
                                let base = colors[0 % colors.len()];
                                let color = egui::Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), a);
                                shapes2.push(egui::Shape::circle_filled(center, radius, color));
                                let (value_label, count) = &group_entries[0];
                                let pct = (*count as f32 / total) * 100.0;
                                let text = format!("{}\n{:.1}%", value_label, pct);
                                let galley_lbl = ui.painter().layout(
                                    text,
                                    egui::FontId::monospace(10.0),
                                    egui::Color32::BLACK,
                                    rect2.width(),
                                );
                                shapes2.push(egui::Shape::galley(center, galley_lbl, egui::Color32::BLACK));
                            } else {
                                for (i, (value_label, count)) in group_entries.iter().enumerate() {
                                    let frac = (*count as f32 / total).max(0.0) as f32;
                                    let end_angle = start_angle + (frac * std::f32::consts::TAU).min(std::f32::consts::TAU - 1e-3);
                                    let base = colors[i % colors.len()];
                                    let color = egui::Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), a);
                                    shapes2.push(egui::Shape::convex_polygon(
                                        pie_slice(center, radius, start_angle, end_angle),
                                        color,
                                        egui::Stroke::NONE,
                                    ));

                                    // label on slice: value and percentage (black text)
                                    let mid = (start_angle + end_angle) * 0.5;
                                    let label_pos = egui::pos2(
                                        center.x + mid.cos() * radius * 0.65,
                                        center.y + mid.sin() * radius * 0.65,
                                    );
                                    let pct = (*count as f32 / total) * 100.0;
                                    let text = format!("{}\n{:.1}%", value_label, pct);
                                    let galley_lbl = ui.painter().layout(
                                        text,
                                        egui::FontId::monospace(10.0),
                                        egui::Color32::BLACK,
                                        rect2.width(),
                                    );
                                    shapes2.push(egui::Shape::galley(label_pos, galley_lbl, egui::Color32::BLACK));
                                    start_angle = end_angle;
                                }
                            }
                            ui.painter().extend(shapes2);
                        });

                        // Legend text mapping: Value X (metric): file1, file2 (scrollable)
                        ui.separator();
                        egui::Frame::none().show(ui, |ui| {
                            ui.set_min_height(140.0);
                            egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                                let mut colors = generate_palette(group_entries.len());
                                let a = (255.0 * self.chart_alpha) as u8;
                                for c in &mut colors { *c = egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a); }
                                for (idx, (value_label, count)) in group_entries.iter().enumerate() {
                                    let files = groups.get(value_label);
                                    let metric_text = match self.stats_metric {
                                        StatsMetric::Percent => format!("{:.1}%", (*count as f32 / total) * 100.0),
                                        StatsMetric::Count => format!("{}", count),
                                    };
                                    let color = colors[idx % colors.len()];
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(egui::RichText::new(format!("Value {} ", value_label)).monospace().strong());
                                        ui.label(egui::RichText::new(format!("({})", metric_text)).color(color).monospace().strong());
                                        if let Some(files) = files {
                                            ui.label(": ");
                                            let file_text = files.join(", ");
                                            ui.add(egui::Label::new(egui::RichText::new(file_text).monospace()).wrap(true));
                                        }
                                    });
                                }
                            });
                        });
                    }
                } else {
                    ui.separator();
                    ui.label("Tip: click a row (Address column) to select.");
                }
            });
        }
    }
}
