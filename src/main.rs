// Copyright (C) 2024 Adrien Bustany <adrien@bustany.org>

use anyhow::{bail, Context, Result};
use gio::prelude::FileExt;

fn usage(arg0: &str) -> String {
    format!(
        concat!(
            "Usage: {} [OPTIONS] file.pdf [output_dir]\n\n",
            "Extracts pages of a PDF as SVG files, and generates a thumbnail for each.\n\n",
            "Options:\n",
            "  --help        : Displays this message and exits\n",
            "  --pages PAGES : Only extract specific pages from the PDF document\n",
            "                  PAGES is a comma separated list of page numbers, where the\n",
            "                  number of the first page is 1."
        ),
        arg0
    )
}

struct Args {
    input_filename: String,
    output_dir: Option<String>,
    page_numbers: Option<Vec<u32>>,
}

fn parse_args() -> Result<Args> {
    let mut pargs = pico_args::Arguments::from_env();

    if pargs.contains("--help") {
        let arg0 = std::env::args().next().unwrap();
        println!("{}", usage(&arg0));
        std::process::exit(0);
    }

    let page_numbers: Option<Vec<u32>> = pargs
        .opt_value_from_fn("--pages", |val| val.split(',').map(|x| x.parse()).collect())
        .context("error parsing page numbers")?;
    let input_filename: String = pargs
        .free_from_str()
        .context("error parsing input filename")?;
    let output_dir = pargs
        .opt_free_from_str()
        .context("error parsing output directory")?;

    Ok(Args {
        input_filename,
        output_dir,
        page_numbers,
    })
}

fn main() -> Result<()> {
    let Args {
        input_filename,
        output_dir,
        page_numbers,
    } = parse_args()?;

    let out_dir = std::path::Path::new(output_dir.as_deref().unwrap_or("."));
    let input_file = gio::File::for_commandline_arg(input_filename);
    let doc =
        poppler::Document::from_file(&input_file.uri(), None).context("error opening PDF file")?;

    let page_count = doc.n_pages();

    if page_count == 0 {
        return Ok(());
    }

    let pages: Box<dyn Iterator<Item = i32>> = if let Some(numbers) = page_numbers {
        Box::new(numbers.into_iter().map(|number| (number as i32) - 1))
    } else {
        Box::new(0..page_count)
    };

    for i in pages {
        if i < 0 || i >= page_count {
            bail!("invalid page number: {}", i);
        }

        let page_number = 1 + i;
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
