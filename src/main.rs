#![allow(non_snake_case)]
//#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use egui::{ProgressBar, ImageButton};
//use egui_dropdown::DropDownBox;

use logfile::Yield;
use std::fs;
use std::path::Path;

use std::sync::{Arc, RwLock};
use std::thread;

//mod time_and_date;
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

//#[derive(Default)]
struct MyApp {
    status: String,
    lang: usize,
    input_path: String,
    log_master: Arc<RwLock<logfile::LogFileHandler>>,

    loading: bool,
    progress_x: Arc<RwLock<u32>>,
    progress_m: Arc<RwLock<u32>>,

    yields: [Yield;3],

    selected_test: usize,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            status: "Ready to go!".to_owned(),
            lang: 0,
            input_path: ".\\log\\".to_owned(),
            log_master: Arc::new(RwLock::new(logfile::LogFileHandler::new())),

            loading: false,
            progress_x: Arc::new(RwLock::new(0)),
            progress_m: Arc::new(RwLock::new(1)),

            yields: [Yield(0,0), Yield(0,0), Yield(0,0)],

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
            ui.horizontal(|ui| {
                ui.set_enabled(!self.loading);
                ui.set_min_width(270.0);

                if ui.button(MESSAGE[INPUT_FOLDER][self.lang]).clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.input_path = path.display().to_string();
                    }
                }

                //ui.text_edit_singleline(&mut self.input_path);

                if ui.button("📁").clicked() && !self.loading {
                    self.loading = true;
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
            });

            if self.loading {

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
                }
            }

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
            
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_enabled(!self.loading);

            {
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
            
        });

    }
}
