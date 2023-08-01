#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use image;
use std::env;
use std::path::Path;

enum SortBy {
    Luminance,
    Hue,
    Saturation,
}

fn threshold_upper_boundary(method: &SortBy) -> u16 {
    match method {
        SortBy::Luminance | SortBy::Saturation => 255,
        SortBy::Hue => 360,
    }
}

fn luminance(pixel: &egui::Color32) -> u16 {
    ((pixel.r() as u16) + (pixel.g() as u16) + (pixel.b() as u16)) / 3
}

fn hue(pixel: &egui::Color32) -> u16 {
    let red = pixel.r() as f32;
    let green = pixel.g() as f32;
    let blue = pixel.b() as f32;

    let min = blue.min(red.min(green));
    let max = blue.max(red.max(green));

    if max == min {
        return 0;
    }

    let hue: f32 = if max == red {
        (green - blue) / (max - min)
    } else if max == green {
        2.0 + (blue - red) / (max - min)
    } else if max == blue {
        4.0 + (red - green) / (max - min)
    } else {
        panic!("how?");
    } * 60.0;

    (if hue < 0.0 { hue + 360.0 } else { hue }) as u16
}

fn saturation(pixel: &egui::Color32) -> u16 {
    let red = pixel.r() as f32 / 255.0;
    let green = pixel.g() as f32 / 255.0;
    let blue = pixel.b() as f32 / 255.0;

    let min = blue.min(red.min(green));
    let max = blue.max(red.max(green));

    if max == min {
        return 0;
    }

    let luminance = (max + min) / 2.0;
    let saturation = 1.0 - ((2.0 * luminance) - 1.0).abs();

    (saturation * 255.0) as u16
}

fn into_intervals(bitmap: Vec<bool>) -> Vec<(usize, usize)> {
    let mut result: Vec<(usize, usize)> = Vec::new();
    let mut interval_start: Option<usize> = None;

    for i in 0..bitmap.len() {
        if bitmap[i] == false && interval_start.is_some() {
            result.push((interval_start.unwrap(), i));
            interval_start = None;
        } else if bitmap[i] == true && interval_start.is_none() {
            interval_start = Some(i);
        }
    }

    if interval_start.is_some() {
        result.push((interval_start.unwrap(), bitmap.len()));
    }

    result
}

fn sort_image(
    lower_threshold: u16,
    higher_threshold: u16,
    image: &mut egui::ColorImage,
    sorting_method: &SortBy,
) {
    let width = image.width();
    let height = image.height();

    let pixel_property = match sorting_method {
        SortBy::Hue => hue,
        SortBy::Saturation => saturation,
        SortBy::Luminance => luminance,
    };

    for yi in 0..height {
        let intervals = {
            let mut pixel_bitmap: Vec<bool> = Vec::with_capacity(width as usize);
            for xi in 0..width {
                let pixel: egui::Color32 = image.pixels[yi * width + xi];
                let value = pixel_property(&pixel);
                let accepted_range = lower_threshold..=higher_threshold;
                pixel_bitmap.push(accepted_range.contains(&value));
            }

            into_intervals(pixel_bitmap)
        };

        for interval in intervals {
            let (start, end) = interval;
            let mut pixels: Vec<egui::Color32> = Vec::with_capacity(end - start);
            for xi in start..end {
                pixels.push(image.pixels[yi * width + xi]);
            }
            pixels.sort_by(|a, b| pixel_property(&a).cmp(&pixel_property(&b)));

            for i in 0..pixels.len() {
                let xi = start + i;
                image.pixels[yi * width + xi] = pixels[i];
            }
        }
    }
}

fn main() {
    let mut args: Vec<String> = env::args().skip(1).collect();

    if args.len() == 0 {
        if gui_main().is_err() {
            std::process::exit(1);
        } else {
            std::process::exit(0);
        }
    } else if args.len() < 4 {
        eprintln!("USAGE: psorter <l/h/s> <lower threshold> <higher threshold> [images]");
        std::process::exit(1);
    }

    let sorting_method = {
        let arg = args.first().expect("ERROR: please choose one of the methods of sorting (l for luminance, h for hue and s for saturation) as a first argument");
        match arg.as_str() {
            "l" => SortBy::Luminance,
            "h" => SortBy::Hue,
            "s" => SortBy::Saturation,
            _ => {
                eprintln!("ERROR: sorting method must be one of the following: l (luminance), h (hue) or s (saturation)");
                std::process::exit(1);
            }
        }
    };
    args.remove(0);

    let lower_threshold = args
        .first()
        .expect("ERROR: please provide lower threshold as a second argument")
        .parse::<u16>()
        .expect("ERROR: threshold must be an integer");
    args.remove(0);

    let higher_threshold = args
        .first()
        .expect("ERROR: please provide higher threshold as a third argument")
        .parse::<u16>()
        .expect("ERROR: threshold must be an integer");
    args.remove(0);

    if lower_threshold > higher_threshold {
        eprintln!("ERROR: lower threshold cannot be bigger than a higher threshold.");
        std::process::exit(1);
    }

    for path in args {
        let mut image = match load_image_from_path(&path) {
            Ok(new_image) => new_image,
            Err(e) => {
                eprintln!("ERROR: cannot load image {}: {}", path, e);
                std::process::exit(1);
            }
        };

        sort_image(
            lower_threshold,
            higher_threshold,
            &mut image,
            &sorting_method,
        );

        let path = Path::new(&path);
        let new_file_name = format!("sorted-{}", path.file_name().unwrap().to_str().unwrap());
        image::save_buffer(
            &new_file_name,
            image.as_raw(),
            image.width() as u32,
            image.height() as u32,
            image::ColorType::Rgba8,
        )
        .expect(&format!("ERROR: failed to save file {}", &new_file_name));
    }
}

fn load_image_from_path(path: &str) -> Result<egui::ColorImage, image::ImageError> {
    let image = image::io::Reader::open(path)?.decode()?;
    let size = [image.width() as _, image.height() as _];
    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_flat_samples();
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        size,
        pixels.as_slice(),
    ))
}

fn save_image(image: &egui::ColorImage, name: &str) {
    let picked_path = if let Some(path) = rfd::FileDialog::new().set_file_name(name).save_file() {
        path.display().to_string()
    } else {
        return;
    };

    image::save_buffer(
        &picked_path,
        image.as_raw(),
        image.width() as u32,
        image.height() as u32,
        image::ColorType::Rgba8,
    )
    .expect(&format!("ERROR: failed to save file {}", &picked_path));
}

// TODO: return a proper error
fn open_image() -> Option<(egui::ColorImage, String)> {
    let picked_path = if let Some(path) = rfd::FileDialog::new().pick_file() {
        path.display().to_string()
    } else {
        return None;
    };

    let image = match load_image_from_path(&picked_path) {
        Ok(new_image) => new_image,
        Err(_) => return None,
    };

    let picked_path = Path::new(&picked_path);

    Some((
        image,
        // all of this just to mimic `basename`
        picked_path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string(),
    ))
}

fn gui_main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1024.0, 1024.0)),
        default_theme: eframe::Theme::Light,
        follow_system_theme: true,
        ..Default::default()
    };

    let mut lower_threshold: u16 = 0;
    let mut higher_threshold: u16 = 255;
    let mut sort_by: SortBy = SortBy::Luminance;
    let mut texture: Option<egui::TextureHandle> = None;
    let mut image = egui::ColorImage::new([512, 512], egui::Color32::TRANSPARENT);
    let mut sorted_image = image.clone();
    let mut changed = true;
    let mut image_name = "placeholder".to_string();

    eframe::run_simple_native("PSORTER", options, move |ctx, _frame| {
        egui::TopBottomPanel::top("my_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(
                    egui::Layout::default().with_cross_align(egui::Align::LEFT),
                    |ui| {
                        ui.horizontal(|ui| {
                            let upper_boundary = threshold_upper_boundary(&sort_by);

                            let mut new_lower_threshold = lower_threshold;
                            ui.label("Lower threshold: ");
                            changed = ui
                                .add(egui::Slider::new(
                                    &mut new_lower_threshold,
                                    0..=upper_boundary,
                                ))
                                .changed()
                                || changed;
                            lower_threshold = new_lower_threshold.clamp(0, higher_threshold);

                            ui.separator();

                            let mut new_higher_threshold = higher_threshold;
                            ui.label("Higher threshold: ");
                            changed = ui
                                .add(egui::Slider::new(
                                    &mut new_higher_threshold,
                                    0..=upper_boundary,
                                ))
                                .changed()
                                || changed;
                            higher_threshold =
                                new_higher_threshold.clamp(lower_threshold, upper_boundary);
                        });
                    },
                );

                ui.with_layout(
                    egui::Layout::default().with_cross_align(egui::Align::RIGHT),
                    |ui| {
                        ui.horizontal(|ui| {
                            if ui.button("Open file…").clicked() {
                                if let Some((new_image, new_image_name)) = open_image() {
                                    texture = Some(ctx.load_texture(
                                        &new_image_name,
                                        new_image.clone(),
                                        Default::default(),
                                    ));
                                    image = new_image;
                                    image_name = new_image_name;

                                    changed = true;
                                }
                            }

                            if ui.button("Save file…").clicked() {
                                save_image(&sorted_image, &image_name);
                            }

                            ui.separator();

                            let luminance_button = ui.add(egui::Button::new("Luminance"));
                            let hue_button = ui.add(egui::Button::new("Hue"));
                            let saturation_button = ui.add(egui::Button::new("Saturation"));

                            if luminance_button.clicked() {
                                sort_by = SortBy::Luminance;
                                changed = true;
                            } else if hue_button.clicked() {
                                sort_by = SortBy::Hue;
                                changed = true;
                            } else if saturation_button.clicked() {
                                sort_by = SortBy::Saturation;
                                changed = true;
                            }

                            match sort_by {
                                SortBy::Luminance => luminance_button,
                                SortBy::Hue => hue_button,
                                SortBy::Saturation => saturation_button,
                            }
                            .highlight();
                        });
                    },
                );
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if texture.is_none() {
                texture = Some(ctx.load_texture(&image_name, image.clone(), Default::default()));
            }

            if changed {
                changed = false;
                sorted_image = image.clone();
                sort_image(
                    lower_threshold,
                    higher_threshold,
                    &mut sorted_image,
                    &sort_by,
                );

                texture =
                    Some(ctx.load_texture(&image_name, sorted_image.clone(), Default::default()));
            }

            if let Some(texture) = texture.as_ref() {
                let available_space = ui.available_size();
                let vertical_scale = available_space.y / (image.height() as f32);
                let horizontal_scale = available_space.x / (image.width() as f32);
                let scale = vertical_scale.min(horizontal_scale);
                let image_size = egui::Vec2::new(
                    (image.width() as f32) * scale,
                    (image.height() as f32) * scale,
                );

                ui.with_layout(
                    egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                    |ui| {
                        ui.image(texture, image_size);
                    },
                );
            } else {
                ui.spinner();
            }
        });
    })
}
