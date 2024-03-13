use common::{
    Config,
    Features,
};

pub fn show(ui: &mut egui::Ui, cfg: &mut Config) {
    ui.separator();

    ui.vertical(|ui| {
        ui.label("Features:");
        for (name, f) in Features::all().iter_names() {
            let mut on = cfg.features.contains(f);
            ui.checkbox(&mut on, name);
            cfg.features.set(f, on);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Fov: ");
        ui.drag_angle(&mut cfg.camera.fov_mut().0);
    });
    ui.vertical(|ui| {
        ui.set_enabled(cfg.features.contains(Features::DISK));

        ui.label("Disk");
        ui.add(
            egui::DragValue::new(&mut cfg.disk.radius)
                .speed(0.1)
                .prefix("Radius: ")
                .clamp_range(0.0..=10.0),
        );
        ui.add(
            egui::DragValue::new(&mut cfg.disk.thickness)
                .speed(0.1)
                .prefix("Thickness: ")
                .clamp_range(0.0..=10.0),
        );
    });

    ui.separator();
}
