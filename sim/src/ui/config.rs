use common::{
    Config,
    Features,
};

pub fn show(ui: &mut egui::Ui, cfg: &mut Config) {
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
        fov_angle(ui, &mut cfg.camera.fov_mut().0);
    });
    ui.vertical(|ui| {
        ui.set_enabled(cfg.features.contains(Features::DISK));

        ui.label("Disk");
        ui.add(egui::Slider::new(&mut cfg.disk.radius, 0.0..=10.0).text("Radius"));
        ui.add(
            egui::Slider::new(&mut cfg.disk.thickness, 0.0..=4.0)
                .logarithmic(true)
                .text("Thickness"),
        );
    });
}

fn fov_angle(ui: &mut egui::Ui, radians: &mut f32) -> egui::Response {
    let mut degrees = radians.to_degrees();
    let drag = egui::DragValue::new(&mut degrees)
        .speed(1.0)
        .suffix("°")
        .clamp_range(30.0..=180.0);

    let mut response = ui.add(drag);

    // only touch `*radians` if we actually changed the degree value
    if degrees != radians.to_degrees() {
        *radians = degrees.to_radians();
        response.changed = true;
    }

    response
}
