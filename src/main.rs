use image::{DynamicImage, GenericImageView, ImageBuffer, Rgb, RgbaImage};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::io::{self, Write};

fn compute_energy(image: &DynamicImage) -> Vec<Vec<f64>> {
    let (width, height) = image.dimensions();
    let mut energy = vec![vec![0.0; height as usize]; width as usize];

    energy.par_iter_mut().enumerate().for_each(|(x, col)| {
        let x_u32 = x as u32;
        for y in 0..height as usize {
            let y_u32 = y as u32;

            let left = if x > 0 {
                image.get_pixel(x_u32 - 1, y_u32)
            } else {
                image.get_pixel(width - 1, y_u32)
            };
            let right = if x < width as usize - 1 {
                image.get_pixel(x_u32 + 1, y_u32)
            } else {
                image.get_pixel(0, y_u32)
            };
            let up = if y > 0 {
                image.get_pixel(x_u32, y_u32 - 1)
            } else {
                image.get_pixel(x_u32, height - 1)
            };
            let down = if y < height as usize - 1 {
                image.get_pixel(x_u32, y_u32 + 1)
            } else {
                image.get_pixel(x_u32, 0)
            };

            let dx = ((right[0] as i32 - left[0] as i32).pow(2)
                + (right[1] as i32 - left[1] as i32).pow(2)
                + (right[2] as i32 - left[2] as i32).pow(2)) as f64;

            let dy = ((down[0] as i32 - up[0] as i32).pow(2)
                + (down[1] as i32 - up[1] as i32).pow(2)
                + (down[2] as i32 - up[2] as i32).pow(2)) as f64;

            col[y] = dx.sqrt() + dy.sqrt();
        }
    });
    energy
}

fn find_seam(energy: &[Vec<f64>]) -> Vec<usize> {
    let width = energy.len();
    let height = energy[0].len();
    let mut dp = vec![vec![f64::INFINITY; height]; width];

    for x in 0..width {
        dp[x][0] = energy[x][0];
    }

    for y in 1..height {
        for x in 0..width {
            let mut min_energy = dp[x][y - 1];

            if x > 0 {
                min_energy = min_energy.min(dp[x - 1][y - 1]);
            }
            if x < width - 1 {
                min_energy = min_energy.min(dp[x + 1][y - 1]);
            }

            dp[x][y] = energy[x][y] + min_energy;
        }
    }

    let mut seam = vec![0; height];
    let mut min_x = 0;
    let mut min_energy = f64::INFINITY;

    for x in 0..width {
        if dp[x][height - 1] < min_energy {
            min_energy = dp[x][height - 1];
            min_x = x;
        }
    }

    seam[height - 1] = min_x;

    for y in (0..height - 1).rev() {
        let x = seam[y + 1];
        let mut best_x = x;
        let mut best_energy = dp[x][y];

        if x > 0 && dp[x - 1][y] < best_energy {
            best_energy = dp[x - 1][y];
            best_x = x - 1;
        }
        if x < width - 1 && dp[x + 1][y] < best_energy {
            best_x = x + 1;
        }

        seam[y] = best_x;
    }

    seam
}

fn remove_seam(image: &DynamicImage, seam: &[usize]) -> RgbaImage {
    let (width, height) = image.dimensions();
    let mut new_image = RgbaImage::new(width - 1, height);

    for y in 0..height {
        let seam_x = seam[y as usize];
        for x in 0..width - 1 {
            let pixel = if (x as usize) >= seam_x {
                image.get_pixel(x + 1, y)
            } else {
                image.get_pixel(x, y)
            };
            new_image.put_pixel(x, y, pixel);
        }
    }
    new_image
}

fn seam_carve(image: &DynamicImage, target_width: u32) -> RgbaImage {
    let mut current_image = image.clone();
    let seams_to_remove = current_image.width() - target_width;

    let pb = ProgressBar::new(seams_to_remove as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta})",
            )
            .unwrap()
            .progress_chars("#>-"),
    );

    for _ in 0..seams_to_remove {
        let energy = compute_energy(&current_image);
        let seam = find_seam(&energy);
        current_image = DynamicImage::ImageRgba8(remove_seam(&current_image, &seam));
        pb.inc(1);
    }

    pb.finish_with_message("Seam carving complete");
    current_image.to_rgba8()
}

fn main() {
    let img = match image::open("input.jpg") {
        Ok(image) => image,
        Err(e) => {
            eprintln!("Error loading image: {}", e);
            return;
        }
    };

    let current_width = img.width();
    println!("Current dimensions: {}x{}", current_width, img.height());

    print!("Enter target width: ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    let target_width: u32 = match input.trim().parse() {
        Ok(w) if w < current_width => w,
        Ok(_) => {
            println!("Target width must be less than current width.");
            return;
        }
        Err(_) => {
            println!("Invalid input. Please enter a number.");
            return;
        }
    };

    println!("Starting seam carving...");
    let carved_image = seam_carve(&img, target_width);

    let rgb_image: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_fn(carved_image.width(), carved_image.height(), |x, y| {
            let pixel = carved_image.get_pixel(x, y);
            Rgb([pixel[0], pixel[1], pixel[2]])
        });

    let output_path = "output.jpg";
    rgb_image.save(output_path).unwrap_or_else(|e| {
        eprintln!("Failed to save image: {}", e);
    });

    println!(
        "Final dimensions: {}x{}",
        carved_image.width(),
        carved_image.height()
    );
    println!("Saved result to {}", output_path);
}
