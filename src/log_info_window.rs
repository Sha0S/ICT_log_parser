use crate::LogFileHandler;
use std::sync::{Arc, RwLock};

pub struct LogInfoWindow {
    enabled: bool,
    DMC: String,
    report: String,

    search_bar: String,
}

impl LogInfoWindow {
    pub fn default() -> Self {
        Self {
            enabled: false,
            DMC: String::new(),
            report: String::new(),
            search_bar: String::new(),
        }
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn open_first_NOK(&mut self, target_DMC: String, lfh: Arc<RwLock<LogFileHandler>>) {
        if let Some(report) = lfh.read().unwrap().get_report_for_SB_NOK(&target_DMC) {
            self.enabled = true;
            self.DMC = target_DMC.clone();
            self.search_bar = target_DMC;
            self.report = report.join("\n");
        }
    }

    pub fn open_w_index(
        &mut self,
        target_DMC: String,
        index: usize,
        lfh: Arc<RwLock<LogFileHandler>>,
    ) {
        if let Some(report) = lfh
            .read()
            .unwrap()
            .get_report_for_SB_w_index(&target_DMC, index)
        {
            self.enabled = true;
            self.DMC = target_DMC.clone();
            self.search_bar = target_DMC;
            self.report = report.join("\n");
        }
    }

    pub fn open(&mut self, target_DMC: String, lfh: Arc<RwLock<LogFileHandler>>) {
        if let Some(report) = lfh.read().unwrap().get_report_for_SB(&target_DMC) {
            self.enabled = true;
            self.DMC = target_DMC.clone();
            self.search_bar = target_DMC;
            self.report = report.join("\n");
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn update(&mut self, ctx: &egui::Context, lfh: Arc<RwLock<LogFileHandler>>) {
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("LIWindow"),
            egui::ViewportBuilder::default()
                .with_title(self.DMC.clone())
                .with_inner_size([400.0, 400.0]),
            |ctx, class| {
                assert!(
                    class == egui::ViewportClass::Immediate,
                    "This egui backend doesn't support multiple viewports"
                );

                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.monospace("DMC:");

                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.search_bar).desired_width(300.0),
                        );

                        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            if let Some(report) =
                                lfh.read().unwrap().get_report_for_SB(&self.search_bar)
                            {
                                self.DMC = self.search_bar.clone();
                                self.report = report.join("\n");
                            }

                            ctx.memory_mut(|mem| mem.request_focus(response.id))
                        }
                    });

                    ui.separator();

                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut self.report.as_str())
                                    .desired_width(f32::INFINITY),
                            );
                        });
                });

                if ctx.input(|i| i.viewport().close_requested()) {
                    self.enabled = false;
                }
            },
        );
    }
}
