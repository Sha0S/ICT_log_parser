

pub struct LogInfoWindow {
    enabled: bool,
    DMC: String,
    report: Vec<String>,
}

impl LogInfoWindow {
    pub fn default() -> Self {
        Self {
            enabled: false,
            DMC: String::new(),
            report: Vec::new(),
        }
    }

    pub fn enable(&mut self, DMC: String, report: Vec<String>) {
        self.enabled = true;
        self.DMC = DMC;
        self.report = report;
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn update(&mut self, ctx: &egui::Context) {
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of(self.DMC.clone()),
            egui::ViewportBuilder::default()
                .with_title(self.DMC.clone())
                .with_inner_size([400.0, 400.0]),
            |ctx, class| {
                assert!(
                    class == egui::ViewportClass::Immediate,
                    "This egui backend doesn't support multiple viewports"
                );

                egui::CentralPanel::default().show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            for rpt in self.report.iter() {
                                ui.label(rpt);
                            }
                        });
                });

                if ctx.input(|i| i.viewport().close_requested()) {
                    self.enabled = false;
                }
            },
        );
    }
}