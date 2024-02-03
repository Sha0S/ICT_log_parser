use chrono::{DateTime, Local};
use std::{ffi::OsString, fs, path::PathBuf};

struct ScannedDir {
    dir: PathBuf,
    changed_files: Vec<(PathBuf, DateTime<Local>)>,
}
pub struct ScanDirWindow {
    enabled: bool,

    // config
    root_dir: PathBuf,
    time_limit: i64,
    whitelist: Vec<OsString>,
    blacklist: Vec<OsString>,

    // result
    scanned_dirs: Vec<ScannedDir>,
}

fn load_vec_from_file(path: &str) -> Vec<OsString> {
    match fs::read_to_string(path) {
        Ok(file) => file.lines().map(|s| s.into()).collect(),
        Err(_) => {
            println!("ERR: cloud not read file: {path}");
            Vec::new()
        }
    }
}

impl ScanDirWindow {
    pub fn default() -> Self {
        ScanDirWindow {
            enabled: false,

            root_dir: PathBuf::from("C:\\Agilent_ICT\\boards\\"),
            time_limit: 7,
            whitelist: load_vec_from_file(".\\res\\whitelist"),
            blacklist: load_vec_from_file(".\\res\\blacklist"),

            scanned_dirs: Vec::new(),
        }
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    fn get_board_directories(&self) -> Result<Vec<PathBuf>, std::io::Error> {
        let mut ret: Vec<PathBuf> = Vec::new();

        for dir in fs::read_dir(&self.root_dir)? {
            let dir = dir?;
            let path = dir.path();
            if path.is_dir() && path.join("testplan").exists() {
                ret.push(path);
            }
        }

        Ok(ret)
    }

    fn get_changed_files(
        &self,
        root: &PathBuf,
        subdir: bool,
    ) -> Result<Vec<(PathBuf, DateTime<Local>)>, std::io::Error> {
        let mut ret: Vec<(PathBuf, DateTime<Local>)> = Vec::new();

        for dir in fs::read_dir(root)? {
            let dir = dir?;
            let path = dir.path();
            if path.is_dir() {
                if self.whitelist.is_empty() || subdir {
                    ret.append(&mut self.get_changed_files(&path, true)?);
                } else {
                    for check in &self.whitelist {
                        if path.ends_with(check) {
                            ret.append(&mut self.get_changed_files(&path, true)?);
                            break;
                        }
                    }
                }
            } else {
                let filename = path.file_name().unwrap();
                if self.blacklist.iter().any(|x| *x == filename) {
                    continue;
                }

                if let Ok(x) = path.metadata() {
                    let modified: DateTime<Local> = x.modified().unwrap().into();
                    if Local::now() - modified < chrono::Duration::days(self.time_limit) {
                        ret.push((path, modified));
                    }
                }
            }
        }

        Ok(ret)
    }

    pub fn update(&mut self, ctx: &egui::Context) {
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("LIWindow"),
            egui::ViewportBuilder::default()
                .with_title("ScanDir")
                .with_inner_size([500.0, 300.0]),
            |ctx, class| {
                assert!(
                    class == egui::ViewportClass::Immediate,
                    "This egui backend doesn't support multiple viewports"
                );

                egui::TopBottomPanel::top("Top").show(ctx, |ui| {
                    if ui.button("Scan!").clicked() {
                        self.scanned_dirs.clear();

                        if let Ok(directories) = self.get_board_directories() {
                            for dir in &directories {
                                match self.get_changed_files(dir, false) {
                                    Ok(files) => {
                                        self.scanned_dirs.push(ScannedDir {
                                            dir: dir.clone(),
                                            changed_files: files,
                                        });
                                    }

                                    Err(err) => {
                                        println!("Err: could not scan directories! {err:?}");
                                    }
                                }
                            }
                        }
                    }
                });

                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            egui::Grid::new("table").show(ui, |ui| {
                                for dir in &self.scanned_dirs {
                                    ui.label(format!("{:?}", dir.dir));
                                    ui.end_row();

                                    for file in &dir.changed_files {
                                        ui.add_space(50.0);
                                        ui.label(format!("{:?}", file.0));
                                        ui.label(format!("{}", file.1.format("%F %R")));

                                        if ui.button("Load").clicked() {
                                            //Do something
                                        }
                                        ui.end_row();
                                    }
                                }
                            });
                        });
                });

                if ctx.input(|i| i.viewport().close_requested()) {
                    self.enabled = false;
                }
            },
        );
    }
}
