use std::path::Path;
use image::{self, Pixel, Rgba};
use std::env;

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

fn sort_image(threshold: u8, path: &str) -> Result<(), image::ImageError> {
    let mut img = image::open(path)?.into_rgba8();
    let (width, height) = img.dimensions();

    for yi in 0..height {
        let intervals = {
            let mut luminance_bitmap: Vec<bool> = Vec::with_capacity(width as usize);
            for xi in 0..width {
                let pixel = img.get_pixel(xi, yi);
                let luminance = pixel.to_luma()[0];
                luminance_bitmap.push(luminance > threshold);
            }

            into_intervals(luminance_bitmap)
        };

        for interval in intervals {
            let (start, end) = interval;
            let mut pixels: Vec<Rgba<u8>> = Vec::with_capacity(end - start);
            for xi in start..end {
                pixels.push(*img.get_pixel(xi as u32, yi));
            }
            pixels.sort_by(|a, b| a.to_luma()[0].cmp(&b.to_luma()[0]));

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
    if args.len() < 2 {
        eprintln!("USAGE: porter <threshold> [images]");
        std::process::exit(1);
    }

    let threshold = args.first().expect("ERROR: please provide threshold (from 0 to 255) as a first argument").parse::<u8>().expect("ERROR: threshold must be in the range from 0 to 255");
    args.remove(0);

    for path in args {
        if sort_image(threshold, &path).is_err() {
            eprintln!("ERROR: Failed to sort image {}.", &path);
        }
    }
}
