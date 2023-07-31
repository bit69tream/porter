#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use image::{self, Pixel, Rgba};
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
        SortBy::Hue => 360
    }
}

fn luminance(pixel: &Rgba<u8>) -> u16 {
    pixel.to_luma()[0] as u16
}

fn hue(pixel: &Rgba<u8>) -> u16 {
    let red = pixel[0] as f32;
    let green = pixel[1] as f32;
    let blue = pixel[2] as f32;

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

fn saturation(pixel: &Rgba<u8>) -> u16 {
    let red = pixel[0] as f32 / 255.0;
    let green = pixel[1] as f32 / 255.0;
    let blue = pixel[2] as f32 / 255.0;

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
    path: &str,
    sorting_method: &SortBy,
) -> Result<(), image::ImageError> {
    let mut img = image::open(path)?.into_rgba8();
    let (width, height) = img.dimensions();

    let pixel_property = match sorting_method {
        SortBy::Hue => hue,
        SortBy::Saturation => saturation,
        SortBy::Luminance => luminance,
    };

    for yi in 0..height {
        let intervals = {
            let mut pixel_bitmap: Vec<bool> = Vec::with_capacity(width as usize);
            for xi in 0..width {
                let pixel = img.get_pixel(xi, yi);
                let value = pixel_property(pixel);
                let accepted_range = lower_threshold..=higher_threshold;
                pixel_bitmap.push(accepted_range.contains(&value));
            }

            into_intervals(pixel_bitmap)
        };

        for interval in intervals {
            let (start, end) = interval;
            let mut pixels: Vec<Rgba<u8>> = Vec::with_capacity(end - start);
            for xi in start..end {
                pixels.push(*img.get_pixel(xi as u32, yi));
            }
            pixels.sort_by(|a, b| pixel_property(&a).cmp(&pixel_property(&b)));

            for i in 0..pixels.len() {
                let xi = start + i;
                img.put_pixel(xi as u32, yi, pixels[i]);
            }
        }
    }

    let path = Path::new(path);
    let file_name = path.file_name().unwrap().to_str().unwrap();

    img.save(format!("sorted-{}", file_name))
}

fn main() {
    let mut args: Vec<String> = env::args().skip(1).collect();

    if args.len() == 0 {
        if gui_main().is_err() {
            std::process::exit(1);
        } else {
            std::process::exit(0);
        }
    } else if args.len() < 3 {
        eprintln!("USAGE: porter <lower threshold> <higher threshold> [images]");
        std::process::exit(1);
    }

    let lower_threshold = args
        .first()
        .expect("ERROR: please provide lower threshold (from 0 to 255) as a first argument")
        .parse::<u16>()
        .expect("ERROR: threshold must be in the range from 0 to 255");
    args.remove(0);

    let higher_threshold = args
        .first()
        .expect("ERROR: please provide higher threshold (from 0 to 255) as a second argument")
        .parse::<u16>()
        .expect("ERROR: threshold must be in the range from 0 to 255");
    args.remove(0);

    if lower_threshold > higher_threshold {
        eprintln!("ERROR: lower threshold cannot be bigger than a higher threshold.");
        std::process::exit(1);
    }

    for path in args {
        if sort_image(lower_threshold, higher_threshold, &path, &SortBy::Luminance).is_err() {
            eprintln!("ERROR: Failed to sort image {}.", &path);
        }
    }
}

fn gui_main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1024.0, 1024.0)),
        default_theme: eframe::Theme::Light,
        resizable: false,
        follow_system_theme: true,
        ..Default::default()
    };

    let mut lower_threshold: u16 = 0;
    let mut higher_threshold: u16 = 255;
    let mut sort_by: SortBy = SortBy::Luminance;

    eframe::run_simple_native("Porter", options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            // ui.style_mut().override_font_id = Some(egui::FontId::new(16.0, egui::FontFamily::Proportional));

            ui.horizontal(|ui| {
                ui.with_layout(
                    egui::Layout::default().with_cross_align(egui::Align::LEFT),
                    |ui| {
                        ui.horizontal(|ui| {
                            let upper_boundary = threshold_upper_boundary(&sort_by);

                            let mut new_lower_threshold = lower_threshold;
                            ui.label("Lower threshold: ");
                            ui.add(egui::Slider::new(&mut new_lower_threshold, 0..=upper_boundary));
                            lower_threshold = new_lower_threshold.clamp(0, higher_threshold);

                            ui.separator();

                            let mut new_higher_threshold = higher_threshold;
                            ui.label("Higher threshold: ");
                            ui.add(egui::Slider::new(&mut new_higher_threshold, 0..=upper_boundary));
                            higher_threshold = new_higher_threshold.clamp(lower_threshold, upper_boundary);
                        });
                    },
                );

                ui.with_layout(
                    egui::Layout::default().with_cross_align(egui::Align::RIGHT),
                    |ui| {
                        ui.horizontal(|ui| {
                            let luminance_button = ui.add(egui::Button::new("Luminance"));
                            let hue_button = ui.add(egui::Button::new("Hue"));
                            let saturation_button = ui.add(egui::Button::new("Saturation"));

                            if luminance_button.clicked() {
                                sort_by = SortBy::Luminance;
                            } else if hue_button.clicked() {
                                sort_by = SortBy::Hue;
                            } else if saturation_button.clicked() {
                                sort_by = SortBy::Saturation;
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
    })
}
