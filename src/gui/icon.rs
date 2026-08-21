//! Assets generator for CopyFile application icon.

/// Generates a 64x64 RGBA icon for the eframe viewport.
#[must_use]
pub fn create_app_icon() -> eframe::egui::IconData {
    let width = 64;
    let height = 64;
    let mut rgba = Vec::with_capacity(width * height * 4);

    for y in 0..height {
        for x in 0..width {
            let cx = x as f32 - 32.0;
            let cy = y as f32 - 32.0;
            let dist = (cx * cx + cy * cy).sqrt();

            // Background rounded badge
            if dist <= 28.0 {
                // Gradient from vibrant cyan/blue to emerald green
                let t = (x as f32 + y as f32) / (width as f32 + height as f32);
                let r = (20.0 * (1.0 - t) + 16.0 * t) as u8;
                let g = (130.0 * (1.0 - t) + 185.0 * t) as u8;
                let b = (246.0 * (1.0 - t) + 129.0 * t) as u8;

                // Center symbol: Dual arrow copy lightning motif
                let in_symbol = ((22..=42).contains(&x) && (20..=24).contains(&y))
                    || ((38..=42).contains(&x) && (20..=36).contains(&y))
                    || ((22..=42).contains(&x) && (40..=44).contains(&y))
                    || ((22..=26).contains(&x) && (28..=44).contains(&y))
                    || ((30..=34).contains(&x) && (26..=38).contains(&y));

                if in_symbol {
                    rgba.extend_from_slice(&[255, 255, 255, 255]); // Pure white
                } else {
                    rgba.extend_from_slice(&[r, g, b, 255]);
                }
            } else {
                // Transparent border
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }

    eframe::egui::IconData {
        rgba,
        width: width as u32,
        height: height as u32,
    }
}
