// Copyright (C) 2024 Adrien Bustany <adrien@bustany.org>

use anyhow::{bail, Context, Result};
use gio::prelude::FileExt;

fn main() -> Result<()> {
    let mut args = std::env::args();
    let arg0 = args.next().unwrap();

    let Some(input_path) = args.next() else {
        bail!("Usage: {} file.pdf [output_dir] [selected_pages]\n\nExtract pages of a PDF as SVG files, and generates a thumbnail for each.", arg0);
    };

    let out_dir_arg = args.next();
    let out_dir = std::path::Path::new(out_dir_arg.as_deref().unwrap_or("."));

    let selected_pages = args.next().map(|s| {
        s.split_whitespace()
            .filter_map(|num| num.parse::<i32>().ok())
            .collect::<Vec<i32>>()
    });

    let input_file = gio::File::for_commandline_arg(input_path);
    let doc =
        poppler::Document::from_file(&input_file.uri(), None).context("error opening PDF file")?;

    let page_count = doc.n_pages();

    if page_count == 0 {
        return Ok(());
    }

    for i in 0..page_count {
        let page_number = 1 + i;

        if let Some(pages) = &selected_pages {
            if !pages.contains(&page_number) {
                continue;
            }
        }

        let page = doc
            .page(i)
            .with_context(|| format!("error accessing page {}", page_number))?;
        let mut page_rect = poppler::Rectangle::new();

        if !page.get_bounding_box(&mut page_rect) {
            bail!("error getting bounding box for page {}", page_number);
        }

        let width = page_rect.x2() - page_rect.x1();
        let height = page_rect.y2() - page_rect.y1();

        render_page(&page, page_number, width, height, out_dir)
            .with_context(|| format!("error rendering page {}", page_number))?;
        render_thumbnail(&page, page_number, width, height, out_dir)
            .with_context(|| format!("error rendering thumbnail for page {}", page_number))?;
    }

    Ok(())
}

fn render_page(
    page: &poppler::Page,
    page_number: i32,
    width: f64,
    height: f64,
    out_dir: &std::path::Path,
) -> Result<()> {
    let svg_filename = out_dir.join(format!("{:03}.svg", page_number));

    let surface = cairo::SvgSurface::new(width, height, Some(&svg_filename))
        .context("error creating SVG surface")?;
    surface.restrict(cairo::SvgVersion::_1_2);
    surface.set_fallback_resolution(150., 150.);
    let ctx = cairo::Context::new(&surface).context("error creating Cairo context")?;
    page.render_for_printing(&ctx);
    ctx.status().context("error rendering page")?;

    Ok(())
}

fn render_thumbnail(
    page: &poppler::Page,
    page_number: i32,
    width: f64,
    height: f64,
    out_dir: &std::path::Path,
) -> Result<()> {
    let (width, height) = (
        check_dimension(width).context("invalid width")?,
        check_dimension(height).context("invalid height")?,
    );
    let ratio = scale_ratio(width, height, 512);
    let (thumb_width, thumb_height) = scale_rect(width, height, ratio);
    let surface = cairo::ImageSurface::create(
        cairo::Format::Rgb24,
        i32::try_from(thumb_width).context("width too big")?,
        i32::try_from(thumb_height).context("height too big")?,
    )
    .context("error creating surface")?;
    surface.set_fallback_resolution(150., 150.);

    {
        let ctx = cairo::Context::new(&surface).context("error creating Cairo context")?;
        ctx.scale(ratio, ratio);
        page.render_for_printing(&ctx);
        ctx.status().context("error rendering page thumbnail")?;
    } // drop context here so that we can access the surface afterwards

    // write the thumbnail to jpeg somehow (using the image crate)

    let buffer = {
        let thumbnail_data: &[u8] = &surface.take_data().context("error accessing image data")?;
        let mut rgb_data: Vec<u8> = vec![0; thumbnail_data.len() - thumbnail_data.len() / 4];

        let mut j: usize = 0;

        for i in (0..thumbnail_data.len()).step_by(4) {
            rgb_data[j] = thumbnail_data[i + 2];
            rgb_data[j + 1] = thumbnail_data[i + 1];
            rgb_data[j + 2] = thumbnail_data[i];
            j += 3;
        }

        image::ImageBuffer::<image::Rgb<u8>, _>::from_vec(thumb_width, thumb_height, rgb_data)
            .unwrap()
    };

    let thumbnail_filename = out_dir.join(format!("{:03}.jpg", page_number));
    buffer
        .save_with_format(&thumbnail_filename, image::ImageFormat::Jpeg)
        .context("error saving thumbnail")?;

    Ok(())
}

fn scale_ratio(w: u32, h: u32, max_size: u32) -> f64 {
    let side = std::cmp::max(w, h);
    if side == 0 {
        return 0.;
    }
    f64::from(max_size) / f64::from(side)
}

fn scale_rect(w: u32, h: u32, ratio: f64) -> (u32, u32) {
    ((f64::from(w) * ratio) as u32, (f64::from(h) * ratio) as u32)
}

fn check_dimension(dim: f64) -> Result<u32> {
    if dim < 0. {
        bail!("value is negative");
    }

    if dim > f64::from(u32::MAX) {
        bail!("value is too large");
    }

    Ok(dim as u32)
}
