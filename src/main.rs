#![allow(non_snake_case)]
//#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use egui::{ProgressBar, ImageButton, RichText, Color32, Vec2};
use egui_extras::{TableBuilder, Column};
use chrono::{NaiveDate, NaiveTime, Timelike};

//use egui_dropdown::DropDownBox;

use logfile::{Yield, FailureList, BResult};
use std::fs;
use std::path::Path;

use std::sync::{Arc, RwLock};
use std::thread;

mod logfile;

include!("locals.rs");

fn count_logs_in_path(p: &Path) -> Result< u32, std::io::Error> {
    let mut i = 0;
    for file in fs::read_dir(p)? {
        let file = file?;
        let path = file.path();
        if path.is_dir() {
            i += count_logs_in_path(&path)?;
        } else {
            i += 1;
        }
    }

    Ok(i)
}

fn read_logs_in_path(b:  Arc<RwLock<logfile::LogFileHandler>>, p: &Path, x_lock: Arc<RwLock<u32>>, frame: egui::Context) -> Result<u32,std::io::Error> {
    println!("INFO: RLiP start at {}", p.display());
    
    for file in fs::read_dir(p)? {
            let file = file?;
            let path = file.path();
            if path.is_dir() {
               let cl = x_lock.clone();
               read_logs_in_path(b.clone(),&path,cl, frame.clone())?;
            } else {
                (*b.write().unwrap()).push_from_file(&path);
                *x_lock.write().unwrap() += 1;
                frame.request_repaint();
            }
    }

    println!("INFO: RLiP end {}", p.display());   
    Ok(0)
}

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default(),
        ..Default::default()
    };

    eframe::run_native(
        "ICT Logfile Parser v2.0",
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
    Hourly
}

//#[derive(Default)]
struct MyApp {
    status: String,
    lang: usize,
    input_path: String,
    log_master: Arc<RwLock<logfile::LogFileHandler>>,

    //#[cfg(feature = "chrono")]
    //#[cfg_attr(feature = "serde", serde(skip))]
    date_start: NaiveDate,
    date_end: NaiveDate,

    time_start: NaiveTime,
    time_start_string: String, 
    time_end: NaiveTime,
    time_end_string: String, 
    time_end_use: bool,

    loading: bool,
    progress_x: Arc<RwLock<u32>>,
    progress_m: Arc<RwLock<u32>>,

    yields: [Yield;3],
    mb_yields: [Yield;3],
    failures: Vec<FailureList>,

    mode: AppMode,

    hourly_stats: Vec<(u64,usize,usize,Vec<(logfile::BResult,u64)>)>,
    selected_test: usize,
}

impl Default for MyApp {
    fn default() -> Self {
        let time_start = chrono::NaiveTime::from_hms_opt(0,0,0).unwrap();
        let time_end = chrono::NaiveTime::from_hms_opt(23,59,59).unwrap();

        Self {
            status: "Ready to go!".to_owned(),
            lang: 0,
            input_path: ".\\log\\".to_owned(),
            log_master: Arc::new(RwLock::new(logfile::LogFileHandler::new())),

            date_start: chrono::Local::now().date_naive(),  
            date_end: chrono::Local::now().date_naive(),

            time_start,
            time_start_string: time_start.format("%H:%M:%S").to_string(),
            time_end,
            time_end_string: time_end.format("%H:%M:%S").to_string(),
            time_end_use: true,

            loading: false,
            progress_x: Arc::new(RwLock::new(0)),
            progress_m: Arc::new(RwLock::new(1)),

            yields: [Yield(0,0), Yield(0,0), Yield(0,0)],
            mb_yields: [Yield(0,0), Yield(0,0), Yield(0,0)],
            failures: Vec::new(),

            mode: AppMode::None,
            hourly_stats: Vec::new(),
            selected_test: 0,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::bottom("Status_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.add(ImageButton::new(egui::include_image!("../res/HU.png"))).clicked() {
                    self.lang = 0;
                    self.status = MESSAGE[LANG_CHANGE][self.lang].to_owned();
                }

                if ui.add(ImageButton::new(egui::include_image!("../res/UK.png"))).clicked() {
                    self.lang = 1;
                    self.status = MESSAGE[LANG_CHANGE][self.lang].to_owned();
                }

                ui.monospace(self.status.to_string());
            });
            
        });

        egui::SidePanel::left("Settings_panel").show(ctx, |ui| {

            // "Menu" bar

            ui.horizontal(|ui| {
                ui.set_enabled(!self.loading);
                ui.set_min_width(270.0);

                if ui.button("📁").clicked() && !self.loading {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.input_path = path.display().to_string();
                    }

                    self.loading = true;
                    self.mode = AppMode::None;
                    self.hourly_stats.clear();
                    self.selected_test = 0;
                    *self.progress_x.write().unwrap() = 0;
                    *self.progress_m.write().unwrap() = 1;

                    let pi=self.input_path.clone();
                    let lb_lock = self.log_master.clone();
                    let pm_lock = self.progress_m.clone();
                    let px_lock = self.progress_x.clone();
                    let frame = ctx.clone();

                    thread::spawn(move || {
                        let p = Path::new(&pi);

                        *pm_lock.write().unwrap() = count_logs_in_path(p).unwrap();
                        //*px_lock.write().unwrap() = 0;
                        (*lb_lock.write().unwrap()).clear();
                        frame.request_repaint();

                        read_logs_in_path(lb_lock.clone(), p, px_lock, frame).expect("Failed to load the logs!");

                        (*lb_lock.write().unwrap()).update();
                        (*lb_lock.write().unwrap()).get_failures();
                    });
                }
                
                if ui.button("💾").clicked() {
                    /*  let path_out = Path::new(&mut self.output_path);
                     let lb = self.log_buffer.read().unwrap();
                     for log in &mut *lb {
                         log.save(path_out);
                     }
                     self.status = "Done!".to_owned();*/
                 }
            
                 //egui::ComboBox::from_label("")
                 /*.selected_text(testlist[self.selected_test].0.to_owned())
                 .show_ui(ui, |ui| {

                     for (i, t) in testlist.iter().enumerate() {
                         ui.selectable_value(&mut self.selected_test, i, t.0.to_owned());
                     }

                 }
                )*/;
            });

            ui.separator();

            // Date and time pickers:

            ui.horizontal(|ui| {

                ui.add(egui_extras::DatePickerButton::new(&mut self.date_start).id_source("Starting time"));

                let response = ui.add(egui::TextEdit::singleline(&mut self.time_start_string).desired_width(100.0));
                if response.lost_focus() {
                    match NaiveTime::parse_from_str( self.time_start_string.as_str(),"%H:%M:%S") {
                        Ok(new_t) => {
                            self.time_start = new_t;
                        }
                        Err(_) => {
                            println!("ERR: Failed to pares time string, reverting!");
                            self.time_start_string = self.time_start.format("%H:%M:%S").to_string();
                        }
                    }
                }

                if ui.button("Load").clicked() {

                }
            });

            ui.horizontal(|ui| {
                ui.horizontal(|ui| {
                    ui.set_enabled(self.time_end_use);

                    ui.add(egui_extras::DatePickerButton::new(&mut self.date_end).id_source("Ending time"));

                    let response = ui.add(egui::TextEdit::singleline(&mut self.time_end_string).desired_width(100.0));
                    if response.lost_focus() {
                        match NaiveTime::parse_from_str( self.time_end_string.as_str(),"%H:%M:%S") {
                            Ok(new_t) => {
                                self.time_end = new_t;
                            }
                            Err(_) => {
                                println!("ERR: Failed to pares time string, reverting!");
                                self.time_end_string = self.time_end.format("%H:%M:%S").to_string();
                            }
                        }
                    }
                });


                ui.checkbox(&mut self.time_end_use, "");
            });

            ui.separator();

            // Shortcuts for common data and time settings:

            ui.horizontal(|ui| { 
                if ui.button("This shift").clicked() {
                    self.date_start = chrono::Local::now().date_naive();
                    self.date_end = chrono::Local::now().date_naive();

                    let time_now = chrono::Local::now().naive_local();
                    let hours_now = time_now.hour();
                    if 6 <= hours_now && hours_now < 14 {
                        self.time_start = chrono::NaiveTime::from_hms_opt(6,0,0).unwrap();
                        self.time_end = chrono::NaiveTime::from_hms_opt(13,59,59).unwrap();
                    } else if 14 <= hours_now && hours_now < 22  {
                        self.time_start = chrono::NaiveTime::from_hms_opt(14,0,0).unwrap();
                        self.time_end = chrono::NaiveTime::from_hms_opt(21,59,59).unwrap();
                    } else {
                        if hours_now < 6 {
                            self.date_start = self.date_start.pred_opt().unwrap(); }
                            self.time_start = chrono::NaiveTime::from_hms_opt(22,0,0).unwrap();
                            self.time_end = chrono::NaiveTime::from_hms_opt(5,59,59).unwrap();
                    }

                    self.time_start_string = self.time_start.format("%H:%M:%S").to_string();
                    self.time_end_string = self.time_end.format("%H:%M:%S").to_string();
                }

                if ui.button("Last 24h").clicked() {
                    self.date_start = chrono::Local::now().date_naive().pred_opt().unwrap();
                    self.time_start = chrono::Local::now().time();
                    self.date_end = chrono::Local::now().date_naive();
                    self.time_end = chrono::Local::now().time();

                    self.time_start_string = self.time_start.format("%H:%M:%S").to_string();
                    self.time_end_string = self.time_end.format("%H:%M:%S").to_string();
                }
            });

            // Loading Bar

            if self.loading {

                ui.separator();

                let mut xx: u32 = 0;
                let mut mm: u32 = 1;

                if let Ok(m) = self.progress_m.try_read() {
                    mm = *m;
                } else {
                    println!("NRA");
                }

                if let Ok(x) = self.progress_x.try_read() {
                    //println!("{}/{}", *x, mm);
                    xx = *x;
                } else {
                    println!("NRA");
                }

                
            
                ui.add(ProgressBar::new(xx as f32 / mm as f32));

                self.status = format!("{}: {} / {}",MESSAGE[LOADING_MESSAGE][self.lang],xx, mm).to_owned();

                if xx == mm {
                    self.loading =  false;

                    // Get Yields
                    self.yields = self.log_master.read().unwrap().get_yields();
                    self.mb_yields = self.log_master.read().unwrap().get_mb_yields();
                    self.failures = self.log_master.read().unwrap().get_failures();
                    self.hourly_stats = self.log_master.read().unwrap().get_hourly_mb_stats();
                }
            }

            // Statistics:

            ui.separator();
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.monospace(MESSAGE[YIELD][self.lang]);
                    ui.monospace(MESSAGE[FIRST_T][self.lang]);
                    ui.monospace(MESSAGE[AFTER_RT][self.lang]);
                    ui.monospace(MESSAGE[TOTAL][self.lang]);
                });

                ui.add(egui::Separator::default().vertical());

                ui.vertical(|ui| {
                    ui.monospace("OK");
                    ui.monospace(format!("{}",self.yields[0].0) );
                    ui.monospace(format!("{}",self.yields[1].0) );
                    ui.monospace(format!("{}",self.yields[2].0) );
                });

                ui.add(egui::Separator::default().vertical());

                ui.vertical(|ui| {
                    ui.monospace("NOK");
                    ui.monospace(format!("{}",self.yields[0].1) );
                    ui.monospace(format!("{}",self.yields[1].1) );
                    ui.monospace(format!("{}",self.yields[2].1) );
                });

                ui.add(egui::Separator::default().vertical());

                ui.vertical(|ui| {
                    ui.monospace("%");
                    ui.monospace(format!("{0:.2}",self.yields[0].precentage()) );
                    ui.monospace(format!("{0:.2}",self.yields[1].precentage()) );
                    ui.monospace(format!("{0:.2}",self.yields[2].precentage()) );
                });
            });

            ui.separator();
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.monospace(MESSAGE[MB_YIELD][self.lang]);
                    ui.monospace(MESSAGE[FIRST_T][self.lang]);
                    ui.monospace(MESSAGE[AFTER_RT][self.lang]);
                    ui.monospace(MESSAGE[TOTAL][self.lang]);
                });

                ui.add(egui::Separator::default().vertical());

                ui.vertical(|ui| {
                    ui.monospace("OK");
                    ui.monospace(format!("{}",self.mb_yields[0].0) );
                    ui.monospace(format!("{}",self.mb_yields[1].0) );
                    ui.monospace(format!("{}",self.mb_yields[2].0) );
                });

                ui.add(egui::Separator::default().vertical());

                ui.vertical(|ui| {
                    ui.monospace("NOK");
                    ui.monospace(format!("{}",self.mb_yields[0].1) );
                    ui.monospace(format!("{}",self.mb_yields[1].1) );
                    ui.monospace(format!("{}",self.mb_yields[2].1) );
                });

                ui.add(egui::Separator::default().vertical());

                ui.vertical(|ui| {
                    ui.monospace("%");
                    ui.monospace(format!("{0:.2}",self.mb_yields[0].precentage()) );
                    ui.monospace(format!("{0:.2}",self.mb_yields[1].precentage()) );
                    ui.monospace(format!("{0:.2}",self.mb_yields[2].precentage()) );
                });
            });

            // Failure list:
            
            if !self.failures.is_empty() {
                ui.vertical(|ui| {
                    ui.separator();

                    TableBuilder::new(ui)
                        .striped(true)
                        .column(Column::initial(200.0).resizable(true))
                        .column(Column::remainder())
                        .header(20.0, |mut header| {
                            header.col(|ui| {
                                ui.heading("Kiesők:");
                            });
                            header.col(|ui| {
                                ui.heading("db");
                            });
                        })
                        .body(|mut body| {
                            for fail in &self.failures {
                                body.row(20.0, |mut row| {
                                    row.col(|ui| {
                                        if ui.button(fail.name.to_owned()).clicked() {
                                            self.selected_test = fail.test_id;
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
            
        // Central panel

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_enabled(!self.loading);

            ui.horizontal(|ui| {
                if ui.button("Plot").clicked() {
                    self.mode = AppMode::Plot;
                }
                if ui.button("Órai").clicked() {
                    self.mode = AppMode::Hourly;
                }
            });

            ui.separator();

            if self.mode == AppMode::Plot {
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

                        }
                    );

                    // Insert plot here


                }
            }

            if self.mode == AppMode::Hourly {
                if !self.hourly_stats.is_empty() {
                    TableBuilder::new(ui)
                    .striped(true)
                    .column(Column::initial(200.0).resizable(true))
                    .column(Column::initial(50.0).resizable(true))
                    .column(Column::initial(50.0).resizable(true))
                    .column(Column::auto().resizable(true))
                    .header(20.0, |mut header| {
                        header.col(|ui| {
                            ui.heading("Time");
                        });
                        header.col(|ui| {
                            ui.heading("OK");
                        });
                        header.col(|ui| {
                            ui.heading("NOK");
                        });
                        header.col(|ui| {
                            ui.heading("Results");
                        });
                    })
                    .body(|mut body| {
                        for hour in &self.hourly_stats {
                            body.row(20.0, |mut row| {
                                row.col(|ui| {
                                    ui.label(u64_to_time(hour.0));
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
                                        for (r,_) in &hour.3 {
                                            ui.label(RichText::new("■").color(
                                                if *r==BResult::Fail { Color32::RED } else { Color32::GREEN }
                                            ));
                                        }
                                    });
                                });
                            });
                        }
                    });
                }
            }
            
        });

    }
}


// Turn YYMMDDHH format u64 int to "YY.MM.DD HH:00 - HH:59"
fn u64_to_time(mut x: u64) -> String {
    let y = x/u64::pow(10, 6);
    x = x % u64::pow(10, 6);

    let m = x/u64::pow(10, 4);
    x = x % u64::pow(10, 4);

    let d = x/u64::pow(10, 2);
    x = x % u64::pow(10, 2);

    format!("{0}.{1}.{2} {3}:00 - {3}:59", y, m, d, x)
}