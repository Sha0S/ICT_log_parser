#![allow(non_snake_case)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use egui::{Color32, ImageButton, Layout, ProgressBar, RichText, Sense, Stroke, Vec2};
use egui_extras::{Column, TableBuilder};
use egui_plot::{uniform_grid_spacer, Bar, BarChart, Line, Plot, PlotPoints};

use chrono::*;

mod logfile;
use logfile::*;

use std::fs;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::thread;

const VERSION: &str = env!("CARGO_PKG_VERSION");
include!("locals.rs");

/*
Currently in the _t functions it checks if the last modification to the files is between the limits.
This wasn't the original behaviour, but it should be fine? It is also really fast.
*/

fn get_logs_in_path(
    p: &Path,
    pm_lock: Arc<RwLock<u32>>,
) -> Result<Vec<(PathBuf, u64)>, std::io::Error> {
    let mut ret: Vec<(PathBuf, u64)> = Vec::new();

    for file in fs::read_dir(p)? {
        let file = file?;
        let path = file.path();
        if path.is_dir() {
            ret.append(&mut get_logs_in_path(&path, pm_lock.clone())?);
        } else if let Ok(x) = path.metadata() {
            ret.push((path.to_path_buf(), x.len()));
            *pm_lock.write().unwrap() += 1;
        }
    }

    Ok(ret)
}

fn is_dir_in_t(s: &Path, start: DateTime<Local>, end: DateTime<Local>) -> bool {
    if let Ok(as_time) =
        NaiveDate::parse_from_str(s.file_name().unwrap().to_str().unwrap(), "%Y_%m_%d")
    {
        if start.date_naive() <= as_time && end.date_naive() >= as_time {
            return true;
        }
    }
    false
}

fn get_logs_in_path_t(
    p: &Path,
    start: DateTime<Local>,
    end: DateTime<Local>,
) -> Result<Vec<(PathBuf, u64)>, std::io::Error> {
    let mut ret: Vec<(PathBuf, u64)> = Vec::new();

    for file in fs::read_dir(p)? {
        let file = file?;
        let path = file.path();
        if path.is_dir() {
            if is_dir_in_t(&path, start, end) {
                ret.append(&mut get_logs_in_path_t(&path, start, end)?);
            }
        } else if let Ok(x) = path.metadata() {
            let ct: DateTime<Local> = x.modified().unwrap().into();
            if ct >= start && ct < end {
                ret.push((path.to_path_buf(), x.len()));
            }
        }
    }

    Ok(ret)
}

// Turn YYMMDDHH format u64 int to "YY.MM.DD HH:00 - HH:59"
fn u64_to_timeframe(mut x: u64) -> String {
    let y = x / u64::pow(10, 6);
    x %= u64::pow(10, 6);

    let m = x / u64::pow(10, 4);
    x %= u64::pow(10, 4);

    let d = x / u64::pow(10, 2);
    x %= u64::pow(10, 2);

    format!(
        "{0:02.0}.{1:02.0}.{2:02.0} {3:02.0}:00 - {3:02.0}:59",
        y, m, d, x
    )
}

struct Product {
    desc: String,
    //id: String,
    path: String,
    //test_folder: String,
}

fn load_product_list() -> Vec<Product> {
    let mut list = Vec::new();

    let p = Path::new(".\\res\\products");
    if let Ok(fileb) = fs::read_to_string(p) {
        for line in fileb.lines() {
            if !line.is_empty() {
                let mut parts = line.split('|');
                let desc = parts.next().unwrap().to_owned();
                let path = parts.next().unwrap().to_owned();

                if Path::new(&path).try_exists().is_ok_and(|x| x) {
                    list.push(Product { desc, path });
                }
            }
        }
    }

    list
}

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default(),
        ..Default::default()
    };

    eframe::run_native(
        format!("ICT Logfile Parser {VERSION}").as_str(),
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Box::<MyApp>::default()
        }),
    )
}

#[derive(PartialEq)]
enum AppMode {
    None,
    Plot,
    Hourly,
    Multiboards,
    Export,
}

#[derive(PartialEq)]
enum YieldMode {
    SingleBoard,
    MultiBoard,
}
enum LoadMode {
    Folder(PathBuf),
    ProductList(String),
}

struct AutoUpdate {
    usable: bool,
    enabled: bool,
    product: Option<usize>,
    last_log: Option<(PathBuf, DateTime<Local>)>,
    last_scan_time: Option<DateTime<Local>>,
}

impl AutoUpdate {
    fn default() -> Self {
        AutoUpdate {
            usable: false,
            enabled: false,
            product: None,
            last_log: None,
            last_scan_time: None,
        }
    }

    fn clear(&mut self) {
        self.usable = false;
        self.enabled = false;
        self.product = None;
        self.last_log = None;
        self.last_scan_time = None;
    }

    fn its_time(&self) -> bool {
        if self.enabled {
            if let Some(t) = self.last_scan_time {
                return (Local::now() - t).num_seconds() > 30;
            }
        }

        false
    }

    /*
        Will have to test it extensively, original implementation had issues when the log are on a different machine, 
        and local time of the host was different. 

        Might want to get every log in the even for x minutes befero the last, and then discard the duplicates.

        It also depends on how good the resolution of "time of modification"  is! If it is too low, this won't be reliable on multiboards!!
    */

    fn get_logs_after_t(
        &self,
        path: &Path,
    ) -> Result<Vec<(PathBuf, DateTime<Local>)>, std::io::Error> {
        let mut ret: Vec<(PathBuf, DateTime<Local>)> = Vec::new();

        // ToDo:
        // Idealy we would get last-log from the last manual load. 
        // That would need the re-write of the fn.
        let start = if let Some((_, x)) = self.last_log{
                x - Duration::seconds(5)
        } else {
            self.last_scan_time.unwrap() - Duration::minutes(5)
            
        };


        for file in fs::read_dir(path)? {
            let file = file?;
            let path = file.path();
            if path.is_file() {
                if let Ok(x) = path.metadata() {
                    let ct: DateTime<Local> = x.modified().unwrap().into();
                    if ct >= start {
                        ret.push((path.to_path_buf(), ct));
                    }
                }
            }
        }

        ret.sort_by_key(|k| k.1);

        Ok(ret)
    }

    fn update(&mut self, products: &[Product], lfh: Arc<RwLock<LogFileHandler>>) {
        if let Some(prod) =
            products.get(self.product.expect("ERR: Auto Updater has no product ID!"))
        {
            if let Ok(logs) = self.get_logs_after_t(Path::new(&prod.path)) {
                for (log, _) in &logs {
                    lfh.write().unwrap().push_from_file(log);
                }

                if let Some(llog) = logs.last() {
                    self.last_log = Some(llog.clone());
                }
            }
        }

        self.last_scan_time = Some(Local::now());
    }
}

struct MyApp {
    status: String,
    lang: usize,
    selected_product: usize,
    product_list: Vec<Product>,
    log_master: Arc<RwLock<LogFileHandler>>,

    date_start: NaiveDate,
    date_end: NaiveDate,

    time_start: NaiveTime,
    time_start_string: String,
    time_end: NaiveTime,
    time_end_string: String,
    time_end_use: bool,

    auto_update: AutoUpdate,

    loading: bool,
    progress_x: Arc<RwLock<u32>>,
    progress_m: Arc<RwLock<u32>>,

    yield_mode: YieldMode,
    yields: [Yield; 3],
    mb_yields: [Yield; 3],
    failures: Vec<FailureList>,
    limitchanges: Option<Vec<(usize, String)>>,

    mode: AppMode,

    hourly_stats: Vec<HourlyStats>,
    multiboard_results: Vec<MbStats>,

    selected_test: usize,
    selected_test_tmp: usize,
    selected_test_results: (TType, Vec<(u64, usize, TResult, TLimit)>),

    export_settings: ExportSettings,
}

impl Default for MyApp {
    fn default() -> Self {
        let time_start = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
        let time_end = NaiveTime::from_hms_opt(23, 59, 59).unwrap();

        Self {
            status: "".to_owned(),
            lang: 0,
            product_list: load_product_list(),
            selected_product: 0,
            log_master: Arc::new(RwLock::new(LogFileHandler::new())),

            date_start: Local::now().date_naive(),
            date_end: Local::now().date_naive(),

            time_start,
            time_start_string: time_start.format("%H:%M:%S").to_string(),
            time_end,
            time_end_string: time_end.format("%H:%M:%S").to_string(),
            time_end_use: true,

            auto_update: AutoUpdate::default(),

            loading: false,
            progress_x: Arc::new(RwLock::new(0)),
            progress_m: Arc::new(RwLock::new(1)),

            yield_mode: YieldMode::SingleBoard,
            yields: [Yield(0, 0), Yield(0, 0), Yield(0, 0)],
            mb_yields: [Yield(0, 0), Yield(0, 0), Yield(0, 0)],
            failures: Vec::new(),
            limitchanges: None,

            mode: AppMode::None,
            hourly_stats: Vec::new(),
            multiboard_results: Vec::new(),

            selected_test: 0,
            selected_test_tmp: 0,
            selected_test_results: (TType::Unknown, Vec::new()),

            export_settings: ExportSettings::default(),
        }
    }
}

impl MyApp {
    fn update_stats(&mut self, ctx: &egui::Context) {
        let mut lock = self.log_master.write().unwrap();

        lock.update();
        self.yields = lock.get_yields();
        self.mb_yields = lock.get_mb_yields();
        self.failures = lock.get_failures();
        self.hourly_stats = lock.get_hourly_mb_stats();
        self.multiboard_results = lock.get_mb_results();
        self.limitchanges = lock.get_tests_w_limit_changes();

        ctx.request_repaint();
    }

    // Do I even need to clear these?
    fn clear_stats(&mut self) {
        self.hourly_stats.clear();
        self.multiboard_results.clear();
        self.auto_update.clear();
        self.selected_test = 0;
        *self.progress_x.write().unwrap() = 0;
        *self.progress_m.write().unwrap() = 1;
    }

    fn load_logs(&mut self, ctx: &egui::Context, mode: LoadMode) {
        //let input_path = product.path.clone();

        let input_path = match mode {
            LoadMode::Folder(ref x) => x.clone(),
            LoadMode::ProductList(ref x) => PathBuf::from(x),
        };

        let start_dt = TimeZone::from_local_datetime(
            &Local,
            &NaiveDateTime::new(self.date_start, self.time_start),
        )
        .unwrap();

        let end_dt = {
            if self.time_end_use {
                TimeZone::from_local_datetime(
                    &Local,
                    &NaiveDateTime::new(self.date_end, self.time_end),
                )
                .unwrap()
            } else {
                Local::now()
            }
        };

        self.loading = true;
        self.clear_stats();

        if matches!(mode, LoadMode::ProductList(_)) && !self.time_end_use {
            self.auto_update.enabled = false;
            self.auto_update.usable = true;
            self.auto_update.product = Some(self.selected_product);
            self.auto_update.last_scan_time = Some(Local::now());
        }

        let lb_lock = self.log_master.clone();
        let pm_lock = self.progress_m.clone();
        let px_lock = self.progress_x.clone();
        let frame = ctx.clone();

        thread::spawn(move || {
            let logs_result = match mode {
                LoadMode::Folder(_) => get_logs_in_path(&input_path, pm_lock.clone()),
                LoadMode::ProductList(_) => get_logs_in_path_t(&input_path, start_dt, end_dt),
            };

            if let Ok(mut logs) = logs_result {
                *pm_lock.write().unwrap() = logs.len() as u32;
                (*lb_lock.write().unwrap()).clear();
                frame.request_repaint_after(std::time::Duration::from_millis(500));

                println!("Found {} logs to load.", logs.len());
                logs.sort_by_key(|k| k.1);

                for log in logs.iter().rev() {
                    (*lb_lock.write().unwrap()).push_from_file(&log.0);
                    *px_lock.write().unwrap() += 1;
                    frame.request_repaint_after(std::time::Duration::from_millis(500));
                }
            }
        });
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(std::time::Duration::from_secs(30));

        egui::SidePanel::left("Settings_panel").show(ctx, |ui| {
            ui.set_min_width(270.0);

            // "Menu" bar
            ui.horizontal(|ui| {
                ui.set_enabled(!self.loading);

                if ui.button("📁").clicked() && !self.loading {
                    if let Some(input_path) = rfd::FileDialog::new().pick_folder() {
                        self.load_logs(ctx, LoadMode::Folder(input_path));
                    }
                }

                egui::ComboBox::from_label("")
                    .width(200.0)
                    .selected_text(match self.product_list.get(self.selected_product) {
                        Some(sel) => sel.desc.clone(),
                        None => "".to_string(),
                    })
                    .show_ui(ui, |ui| {
                        for (i, t) in self.product_list.iter().enumerate() {
                            ui.selectable_value(&mut self.selected_product, i, t.desc.clone());
                        }
                    });
            });

            ui.separator();

            // Date and time pickers:
            ui.horizontal(|ui| {
                ui.add(
                    egui_extras::DatePickerButton::new(&mut self.date_start)
                        .id_source("Starting time"),
                );

                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.time_start_string).desired_width(70.0),
                );
                if response.lost_focus() {
                    match NaiveTime::parse_from_str(self.time_start_string.as_str(), "%H:%M:%S") {
                        Ok(new_t) => {
                            self.time_start = new_t;
                        }
                        Err(_) => {
                            println!("ERR: Failed to pares time string, reverting!");
                            self.time_start_string = self.time_start.format("%H:%M:%S").to_string();
                        }
                    }
                }

                // Set timeframe to this shift
                if ui.button(MESSAGE[SHIFT][self.lang]).clicked() {
                    self.date_start = Local::now().date_naive();
                    self.date_end = Local::now().date_naive();

                    let time_now = Local::now().naive_local();
                    let hours_now = time_now.hour();
                    if (6..14).contains(&hours_now) {
                        self.time_start = NaiveTime::from_hms_opt(6, 0, 0).unwrap();
                        self.time_end = NaiveTime::from_hms_opt(13, 59, 59).unwrap();
                    } else if (14..22).contains(&hours_now) {
                        self.time_start = NaiveTime::from_hms_opt(14, 0, 0).unwrap();
                        self.time_end = NaiveTime::from_hms_opt(21, 59, 59).unwrap();
                    } else {
                        if hours_now < 6 {
                            self.date_start = self.date_start.pred_opt().unwrap();
                        }
                        self.time_start = NaiveTime::from_hms_opt(22, 0, 0).unwrap();
                        self.time_end = NaiveTime::from_hms_opt(5, 59, 59).unwrap();
                    }

                    self.time_start_string = self.time_start.format("%H:%M:%S").to_string();
                    self.time_end_string = self.time_end.format("%H:%M:%S").to_string();
                }

                // Set timeframe to the last 24h
                if ui.button(MESSAGE[A_DAY][self.lang]).clicked() {
                    self.date_start = Local::now().date_naive().pred_opt().unwrap();
                    self.time_start = Local::now().time();
                    self.date_end = Local::now().date_naive();
                    self.time_end = Local::now().time();

                    self.time_start_string = self.time_start.format("%H:%M:%S").to_string();
                    self.time_end_string = self.time_end.format("%H:%M:%S").to_string();
                }
            });

            ui.horizontal(|ui| {
                ui.horizontal(|ui| {
                    ui.set_enabled(self.time_end_use);

                    ui.add(
                        egui_extras::DatePickerButton::new(&mut self.date_end)
                            .id_source("Ending time"),
                    );

                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.time_end_string).desired_width(70.0),
                    );
                    if response.lost_focus() {
                        match NaiveTime::parse_from_str(self.time_end_string.as_str(), "%H:%M:%S") {
                            Ok(new_t) => {
                                self.time_end = new_t;
                            }
                            Err(_) => {
                                println!("ERR: Failed to parse time string, reverting!");
                                self.time_end_string = self.time_end.format("%H:%M:%S").to_string();
                            }
                        }
                    }
                });

                ui.add(egui::Checkbox::without_text(&mut self.time_end_use));

                if ui.button(MESSAGE[LOAD][self.lang]).clicked() && !self.loading {
                    if let Some(product) = self.product_list.get(self.selected_product) {
                        self.load_logs(ctx, LoadMode::ProductList(product.path.clone()));
                    }
                }
            });

            // Auto-update checkbox
            ui.horizontal(|ui| {
                ui.set_enabled(self.auto_update.usable);

                ui.monospace(MESSAGE[AUTO_UPDATE][self.lang]);
                ui.add(egui::Checkbox::without_text(&mut self.auto_update.enabled));
            });

            // Loading Bar
            if self.loading {
                ui.separator();

                let mut xx: u32 = 0;
                let mut mm: u32 = 1;

                if let Ok(m) = self.progress_m.try_read() {
                    mm = *m;
                }
                if let Ok(x) = self.progress_x.try_read() {
                    xx = *x;
                }

                ui.add(
                    ProgressBar::new(xx as f32 / mm as f32)
                        .text(RichText::new(format!("{} / {}", xx, mm)))
                        .animate(true),
                );

                self.status =
                    format!("{}: {} / {}", MESSAGE[LOADING_MESSAGE][self.lang], xx, mm).to_owned();

                if xx == mm {
                    self.loading = false;
                    self.update_stats(ctx);
                }
            } else if self.auto_update.its_time() {
                self.auto_update.update(&self.product_list, self.log_master.clone());
                self.update_stats(ctx)
            }
            

            // Statistics:
            ui.separator();

            ui.horizontal(|ui| {
                ui.monospace(MESSAGE[YIELD][self.lang]);

                // Localiazation?
                ui.selectable_value(&mut self.yield_mode, YieldMode::SingleBoard, "Single");
                ui.selectable_value(&mut self.yield_mode, YieldMode::MultiBoard, "Multiboard");
            });

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.monospace("");
                    ui.monospace(MESSAGE[FIRST_T][self.lang]);
                    ui.monospace(MESSAGE[AFTER_RT][self.lang]);
                    ui.monospace(MESSAGE[TOTAL][self.lang]);
                });

                ui.add(egui::Separator::default().vertical());

                let x = match self.yield_mode {
                    YieldMode::SingleBoard => &self.yields,
                    YieldMode::MultiBoard => &self.mb_yields,
                };

                ui.vertical(|ui| {
                    ui.monospace("OK");
                    ui.monospace(format!("{}", x[0].0));
                    ui.monospace(format!("{}", x[1].0));
                    ui.monospace(format!("{}", x[2].0));
                });

                ui.add(egui::Separator::default().vertical());

                ui.vertical(|ui| {
                    ui.monospace("NOK");
                    ui.monospace(format!("{}", x[0].1));
                    ui.monospace(format!("{}", x[1].1));
                    ui.monospace(format!("{}", x[2].1));
                });

                ui.add(egui::Separator::default().vertical());

                ui.vertical(|ui| {
                    ui.monospace("%");
                    ui.monospace(format!("{0:.2}", x[0].precentage()));
                    ui.monospace(format!("{0:.2}", x[1].precentage()));
                    ui.monospace(format!("{0:.2}", x[2].precentage()));
                });
            });

            // Failure list:
            if !self.failures.is_empty() {
                ui.vertical(|ui| {
                    ui.separator();

                    TableBuilder::new(ui)
                        .striped(true)
                        .column(Column::initial(220.0).resizable(true))
                        .column(Column::remainder())
                        .header(20.0, |mut header| {
                            header.col(|ui| {
                                ui.heading(MESSAGE[FAILURES][self.lang]);
                            });
                            header.col(|ui| {
                                ui.heading(MESSAGE[PCS][self.lang]);
                            });
                        })
                        .body(|mut body| {
                            for fail in &self.failures {
                                body.row(16.0, |mut row| {
                                    row.col(|ui| {
                                        if ui
                                            .add(
                                                egui::Label::new(fail.name.to_owned())
                                                    .truncate(true)
                                                    .sense(Sense::click()),
                                            )
                                            .clicked()
                                        {
                                            self.selected_test = fail.test_id;
                                            self.mode = AppMode::Plot;
                                        }
                                    });
                                    row.col(|ui| {
                                        ui.label(format!("{}", fail.total));
                                    });
                                });
                            }
                        });
                });
            }
        });

        // Status panel + language change
        egui::TopBottomPanel::bottom("Status_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .add(ImageButton::new(egui::include_image!("../res/HU.png")))
                    .clicked()
                {
                    self.lang = LANG_HU;
                    self.status = MESSAGE[LANG_CHANGE][self.lang].to_owned();
                }

                if ui
                    .add(ImageButton::new(egui::include_image!("../res/UK.png")))
                    .clicked()
                {
                    self.lang = LANG_EN;
                    self.status = MESSAGE[LANG_CHANGE][self.lang].to_owned();
                }

                ui.monospace(self.status.to_string());
            });
        });

        // Failed DMC list for Plot view - needs its own panel!
        if self.mode == AppMode::Plot && !self.failures.is_empty() {
            if let Some(x) = self
                .failures
                .iter()
                .find(|k| k.test_id == self.selected_test)
            {
                egui::TopBottomPanel::bottom("failed panels")
                    .resizable(true)
                    .show(ctx, |ui| {
                        ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
                            TableBuilder::new(ui)
                                .striped(true)
                                .column(Column::initial(150.0).resizable(true))
                                .column(Column::initial(100.0).resizable(true))
                                .body(|mut body| {
                                    for fail in &x.failed {
                                        body.row(20.0, |mut row| {
                                            row.col(|ui| {
                                                ui.label(fail.0.to_string());
                                            });
                                            row.col(|ui| {
                                                ui.label(u64_to_string(fail.1));
                                            });
                                        });
                                    }
                                });

                            if x.by_index.len() > 1 {
                                let mut bars: Vec<Bar> = Vec::new();
                                for bar in x.by_index.iter().enumerate() {
                                    bars.push(Bar {
                                        name: format!("{}.", bar.0 as u64 + 1),
                                        orientation: egui_plot::Orientation::Vertical,
                                        argument: bar.0 as f64 + 1.0,
                                        value: *bar.1 as f64,
                                        base_offset: None,
                                        bar_width: 0.5,
                                        stroke: Stroke {
                                            width: 1.0,
                                            color: Color32::GRAY,
                                        },
                                        fill: Color32::RED,
                                    });
                                }
                                let chart = BarChart::new(bars);

                                Plot::new("failure by index")
                                    .show_x(false)
                                    .show_y(false)
                                    .allow_scroll(false)
                                    .allow_drag(false)
                                    .allow_boxed_zoom(false)
                                    .clamp_grid(true)
                                    .set_margin_fraction(Vec2 { x: 0.05, y: 0.1 })
                                    .width(std::cmp::max(8, x.by_index.len()) as f32 * 30.0)
                                    .show(ui, |ui| {
                                        ui.bar_chart(chart);
                                    });
                            }
                        });
                    });
            }
        }

        // Central panel
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_enabled(!self.loading);

            // Top "menu bar"
            ui.horizontal(|ui| {
                if ui.button(MESSAGE_E[EXPORT_LABEL][self.lang]).clicked() {
                    self.mode = AppMode::Export;
                }

                if ui.button(MESSAGE_H[HOURLY_LABEL][self.lang]).clicked() {
                    self.mode = AppMode::Hourly;
                }

                if ui.button(MESSAGE_H[MULTI_LABEL][self.lang]).clicked() {
                    self.mode = AppMode::Multiboards;
                }

                if ui.button(MESSAGE_P[PLOT_LABEL][self.lang]).clicked() {
                    self.mode = AppMode::Plot;
                }
            });

            ui.separator();

            // Plot mode
            if self.mode == AppMode::Plot && !self.loading {
                let lfh = self.log_master.read().unwrap();
                let testlist = lfh.get_testlist();
                if !testlist.is_empty() {
                    // I will need to replace this latter with something edittable
                    egui::ComboBox::from_label("")
                        .selected_text(testlist[self.selected_test].0.to_owned())
                        .show_ui(ui, |ui| {
                            for (i, t) in testlist.iter().enumerate() {
                                ui.selectable_value(&mut self.selected_test, i, t.0.to_owned());
                            }
                        });

                    ui.separator();

                    if self.selected_test != self.selected_test_tmp
                        || self.selected_test_results.1.is_empty()
                    {
                        println!("INFO: Loading results for test nbr {}!", self.selected_test);
                        self.selected_test_results = lfh.get_stats_for_test(self.selected_test);
                        if self.selected_test_results.1.is_empty() {
                            println!("\tERR: Loading failed!");
                            self.selected_test = self.selected_test_tmp;
                        } else {
                            println!("\tINFO: Loading succefull!");
                            self.selected_test_tmp = self.selected_test;
                        }
                    }

                    // Insert plot here

                    let ppoints: PlotPoints = self
                        .selected_test_results
                        .1
                        .iter()
                        .filter_map(|r| {
                            if r.2 .0 == BResult::Unknown {
                                return None;
                            }
                            Some([r.0 as f64, r.2 .1 as f64])
                        })
                        .collect();

                    //Lim2 (f32,f32),     // UL - LL
                    //Lim3 (f32,f32,f32)  // Nom - UL - LL
                    let upper_limit_p: PlotPoints = self
                        .selected_test_results
                        .1
                        .iter()
                        .filter_map(|r| {
                            if let TLimit::Lim3(_, x, _) = r.3 {
                                Some([r.0 as f64, x as f64])
                            } else if let TLimit::Lim2(x, _) = r.3 {
                                Some([r.0 as f64, x as f64])
                            } else {
                                None
                            }
                        })
                        .collect();

                    let nominal_p: PlotPoints = self
                        .selected_test_results
                        .1
                        .iter()
                        .filter_map(|r| {
                            if let TLimit::Lim3(x, _, _) = r.3 {
                                Some([r.0 as f64, x as f64])
                            } else {
                                None
                            }
                        })
                        .collect();

                    let lower_limit_p: PlotPoints = self
                        .selected_test_results
                        .1
                        .iter()
                        .filter_map(|r| {
                            if let TLimit::Lim3(_, _, x) = r.3 {
                                Some([r.0 as f64, x as f64])
                            } else if let TLimit::Lim2(_, x) = r.3 {
                                Some([r.0 as f64, x as f64])
                            } else {
                                None
                            }
                        })
                        .collect();

                    let points = egui_plot::Points::new(ppoints)
                        .highlight(true)
                        .color(Color32::BLUE)
                        .name(testlist[self.selected_test].0.to_owned());

                    let upper_limit = Line::new(upper_limit_p).color(Color32::RED).name("MAX");

                    let nominal = Line::new(nominal_p).color(Color32::GREEN).name("Nom");

                    let lower_limit = Line::new(lower_limit_p).color(Color32::RED).name("MIN");

                    Plot::new("Test results")
                        //.auto_bounds_x()
                        //.auto_bounds_y()
                        .custom_x_axes(vec![egui_plot::AxisHints::default()
                            .formatter(x_formatter)
                            .label("time")])
                        .custom_y_axes(vec![egui_plot::AxisHints::default()
                            .formatter(y_formatter)
                            .label(match self.selected_test_results.0 {
                                TType::Capacitor => "F",
                                TType::Resistor => "Ω",
                                TType::Jumper => "Ω",
                                TType::Fuse => "Ω",
                                TType::Inductor => "H",
                                TType::Diode => "V",
                                TType::Zener => "V",
                                TType::Measurement => "V",
                                _ => "Result",
                            })])
                        .coordinates_formatter(
                            egui_plot::Corner::RightTop,
                            egui_plot::CoordinatesFormatter::new(c_formater),
                        )
                        .label_formatter(|name, value| {
                            if !name.is_empty() {
                                format!("{}: {:+1.4E}", name, value.y)
                            } else {
                                "".to_owned()
                            }
                        })
                        .x_grid_spacer(uniform_grid_spacer(|x| {
                            if x.base_step_size < 150.0 {
                                [3600.0 * 4.0, 3600.0, 900.0]
                            } else if x.base_step_size < 600.0 {
                                [3600.0 * 8.0, 3600.0 * 4.0, 3600.0]
                            } else if x.base_step_size < 2400.0 {
                                [3600.0 * 32.0, 3600.0 * 16.0, 3600.0 * 4.0]
                            } else {
                                [3600.0 * 24.0 * 30.0, 3600.0 * 24.0 * 7.0, 3600.0 * 24.0]
                            }
                        }))
                        .show(ui, |plot_ui| {
                            plot_ui.points(points);
                            plot_ui.line(upper_limit);
                            plot_ui.line(nominal);
                            plot_ui.line(lower_limit);
                        });
                }
            }

            // Hourly mode
            if self.mode == AppMode::Hourly && !self.hourly_stats.is_empty() {
                ui.push_id("hourly", |ui| {
                    TableBuilder::new(ui)
                        .striped(true)
                        .column(Column::initial(150.0).resizable(true))
                        .column(Column::initial(50.0).resizable(true))
                        .column(Column::initial(50.0).resizable(true))
                        .column(Column::auto().resizable(true))
                        .header(20.0, |mut header| {
                            header.col(|ui| {
                                ui.heading(MESSAGE_H[TIME][self.lang]);
                            });
                            header.col(|ui| {
                                ui.heading("OK");
                            });
                            header.col(|ui| {
                                ui.heading("NOK");
                            });
                            header.col(|ui| {
                                ui.heading(MESSAGE_H[RESULTS][self.lang]);
                            });
                        })
                        .body(|mut body| {
                            for hour in &self.hourly_stats {
                                body.row(15.0, |mut row| {
                                    row.col(|ui| {
                                        ui.label(u64_to_timeframe(hour.0));
                                    });
                                    row.col(|ui| {
                                        ui.label(format!("{}", hour.1));
                                    });
                                    row.col(|ui| {
                                        ui.label(format!("{}", hour.2));
                                    });
                                    row.col(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing = Vec2::new(1.0, 1.0);
                                            for (r, _) in &hour.3 {
                                                ui.label(RichText::new("■").color(r.into_color()));
                                            }
                                        });
                                    });
                                });
                            }
                        });
                });
            }

            // Multiboards mode
            if self.mode == AppMode::Multiboards && !self.multiboard_results.is_empty() {
                ui.push_id("multib", |ui| {
                    TableBuilder::new(ui)
                        .striped(true)
                        .column(Column::initial(40.0).resizable(true))
                        .column(Column::initial(200.0).resizable(true))
                        .column(Column::initial(130.0).resizable(true))
                        .column(Column::auto().resizable(true))
                        .body(|mut body| {
                            for (i, mb) in self.multiboard_results.iter().enumerate() {
                                let color_mb = mb.1.last().unwrap().1.into_dark_color();
                                for (i2, sb) in mb.1.iter().enumerate() {
                                    let color_sb = sb.1.into_dark_color();
                                    body.row(15.0, |mut row| {
                                        row.col(|ui| {
                                            if i2 == 0 {
                                                //ui.label(format!("{}.", i+1));
                                                ui.label(
                                                    egui::RichText::new(format!("{}.", i + 1))
                                                        .color(color_mb),
                                                );
                                            }
                                        });
                                        row.col(|ui| {
                                            if i2 == 0 {
                                                //ui.label(mb.0.clone());
                                                ui.label(
                                                    egui::RichText::new(mb.0.clone())
                                                        .color(color_mb),
                                                );
                                            }
                                        });
                                        row.col(|ui| {
                                            //ui.label(u64_to_string( sb.0));
                                            ui.label(
                                                egui::RichText::new(u64_to_string(sb.0))
                                                    .color(color_sb),
                                            );
                                        });
                                        row.col(|ui| {
                                            ui.horizontal(|ui| {
                                                ui.spacing_mut().item_spacing = Vec2::new(1.0, 1.0);
                                                for r in &sb.2 {
                                                    ui.label(
                                                        RichText::new("■").color(r.into_color()),
                                                    );
                                                }
                                            });
                                        });
                                    });
                                }
                            }
                        });
                });
            }

            // Export mode
            if self.mode == AppMode::Export {
                ui.heading(MESSAGE_E[SETTINGS][self.lang]);
                ui.checkbox(
                    &mut self.export_settings.vertical,
                    MESSAGE_E[VERTICAL_O][self.lang],
                );
                ui.checkbox(
                    &mut self.export_settings.only_failed_panels,
                    MESSAGE_E[EXPORT_NOK_ONLY][self.lang],
                );
                ui.horizontal(|ui| {
                    ui.monospace(MESSAGE_E[EXPORT_MODE][self.lang]);
                    ui.selectable_value(
                        &mut self.export_settings.mode,
                        ExportMode::All,
                        MESSAGE_E[EXPORT_MODE_ALL][self.lang],
                    );
                    ui.selectable_value(
                        &mut self.export_settings.mode,
                        ExportMode::FailuresOnly,
                        MESSAGE_E[EXPORT_MODE_FTO][self.lang],
                    );
                    ui.selectable_value(
                        &mut self.export_settings.mode,
                        ExportMode::Manual,
                        MESSAGE_E[EXPORT_MODE_MANUAL][self.lang],
                    );
                });

                if self.export_settings.mode == ExportMode::Manual {
                    ui.monospace(MESSAGE_E[EXPORT_MANUAL][self.lang]);
                    ui.text_edit_singleline(&mut self.export_settings.list);
                    ui.monospace(MESSAGE_E[EXPORT_MANUAL_EX][self.lang]);
                }

                ui.separator();

                if ui.button(MESSAGE_E[SAVE][self.lang]).clicked() && !self.loading {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("XLSX", &["xlsx"])
                        .set_file_name("out.xlsx")
                        .save_file()
                    {
                        self.log_master
                            .read()
                            .unwrap()
                            .export(path, &self.export_settings);
                    }
                }

                // If there are tests with limit changes, then notify the user
                if let Some(changed_tests) = &self.limitchanges {
                    ui.add_space(10.0);
                    for (id, name) in changed_tests {
                        if ui
                            .add(
                                egui::Label::new(
                                    egui::RichText::new(format!(
                                        "{} {} {}",
                                        MESSAGE_E[LIMIT_W][self.lang],
                                        name,
                                        MESSAGE_E[LIMIT_W2][self.lang]
                                    ))
                                    .color(Color32::RED)
                                    .size(14.0),
                                )
                                .sense(Sense::click()),
                            )
                            .clicked()
                        {
                            self.selected_test = *id;
                            self.mode = AppMode::Plot;
                        }
                    }
                }
            }
        });
    }
}

// Formaters for the plot

fn y_formatter(tick: f64, _max_digits: usize, _range: &RangeInclusive<f64>) -> String {
    format!("{tick:+1.1E}")
}

fn x_formatter(tick: f64, _max_digits: usize, _range: &RangeInclusive<f64>) -> String {
    let h = tick / 3600.0;
    let m = (tick % 3600.0) / 60.0;
    let s = tick % 60.0;
    format!("{h:02.0}:{m:02.0}:{s:02.0}")
}

fn c_formater(point: &egui_plot::PlotPoint, _: &egui_plot::PlotBounds) -> String {
    let h = point.x / 3600.0;
    let m = (point.x % 3600.0) / 60.0;
    let s = point.x % 60.0;

    format!("x: {:+1.4E}\t t: {h:02.0}:{m:02.0}:{s:02.0}", point.y)
}
