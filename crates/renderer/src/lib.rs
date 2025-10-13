use screenshot_core::{Annotation, AnnotationKind, BlendMode, Frame, LineStyle, PixelFormat};

/// 简单 RGBA 图像结构
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>, // RGBA
}
impl Image {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; (width * height * 4) as usize],
        }
    }

    #[inline]
    fn idx(&self, x: u32, y: u32) -> usize {
        ((y * self.width + x) * 4) as usize
    }

    pub fn fill_rgba(&mut self, r: u8, g: u8, b: u8, a: u8) {
        for p in self.pixels.chunks_exact_mut(4) {
            p[0] = r;
            p[1] = g;
            p[2] = b;
            p[3] = a;
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, r: u8, g: u8, b: u8, a: u8) {
        let (w0, h0) = (self.width as i32, self.height as i32);
        let x2 = (x + w).min(w0);
        let y2 = (y + h).min(h0);
        let xs = x.max(0);
        let ys = y.max(0);
        for yy in ys..y2 {
            for xx in xs..x2 {
                let i = self.idx(xx as u32, yy as u32);
                self.pixels[i..i + 4].copy_from_slice(&[r, g, b, a]);
            }
        }
    }
}

/// 渲染器：根据注解与原始尺寸输出 RGBA 像素
pub trait Renderer {
    fn render(&self, frame: &Frame, annotations: &[Annotation]) -> Image;
}

pub struct SimpleRenderer;
impl Renderer for SimpleRenderer {
    fn render(&self, frame: &Frame, annotations: &[Annotation]) -> Image {
        // 复制底图 (BGRA 或 RGBA) 到 Image
        let mut img = Image::new(frame.width, frame.height);
        match frame.pixel_format {
            PixelFormat::Rgba8 => {
                img.pixels.copy_from_slice(&frame.bytes);
            }
            PixelFormat::Bgra8 => {
                for (dst, chunk) in img
                    .pixels
                    .chunks_exact_mut(4)
                    .zip(frame.bytes.chunks_exact(4))
                {
                    dst[0] = chunk[2]; // R
                    dst[1] = chunk[1]; // G
                    dst[2] = chunk[0]; // B
                    dst[3] = chunk[3]; // A
                }
            }
        }
        let base_pixels = img.pixels.clone(); // Mosaic 采样使用原始拷贝
        let mut anns: Vec<&Annotation> = annotations.iter().collect();
        anns.sort_by_key(|a| a.meta.z);
        for ann in anns {
            match &ann.kind {
                AnnotationKind::Rect { .. } => {
                    let meta = &ann.meta;
                    let opacity = meta.opacity.clamp(0.0, 1.0);
                    if let Some(fill) = &meta.fill_color {
                        if let Some((r, g, b)) = parse_hex_color(fill) {
                            blend_fill_rect(
                                &mut img,
                                meta.x as i32,
                                meta.y as i32,
                                meta.w as i32,
                                meta.h as i32,
                                r,
                                g,
                                b,
                                (255.0 * opacity) as u8,
                            );
                        }
                    }
                    if let Some(width) = meta.stroke_width {
                        if let Some(stroke_color) = &meta.stroke_color {
                            if let Some((r, g, b)) = parse_hex_color(stroke_color) {
                                stroke_rect(
                                    &mut img,
                                    meta.x as i32,
                                    meta.y as i32,
                                    meta.w as i32,
                                    meta.h as i32,
                                    width,
                                    r,
                                    g,
                                    b,
                                    (255.0 * opacity) as u8,
                                );
                            }
                        }
                    }
                }
                AnnotationKind::Highlight { mode } => {
                    let meta = &ann.meta;
                    if let Some(fill) = &meta.fill_color {
                        if let Some((r, g, b)) = parse_hex_color(fill) {
                            let blend_mode = match mode {
                                BlendMode::Multiply => Blend::Multiply,
                                BlendMode::Screen => Blend::Screen,
                            };
                            highlight_rect(
                                &mut img,
                                meta.x as i32,
                                meta.y as i32,
                                meta.w as i32,
                                meta.h as i32,
                                r,
                                g,
                                b,
                                (255.0 * meta.opacity.clamp(0.0, 1.0)) as u8,
                                blend_mode,
                            );
                        }
                    }
                }
                AnnotationKind::Arrow {
                    head_size,
                    line_style,
                } => {
                    let m = &ann.meta;
                    let (x1, y1) = (m.x as i32, m.y as i32);
                    let (x2, y2) = ((m.x + m.w) as i32, (m.y + m.h) as i32);
                    let width_px = m.stroke_width.unwrap_or(2.0).max(1.0) as i32;
                    let color = m
                        .stroke_color
                        .as_ref()
                        .and_then(|c| parse_hex_color(c))
                        .unwrap_or((255, 255, 255));
                    if let LineStyle::Dashed = line_style {
                        draw_dashed_line(
                            &mut img,
                            x1,
                            y1,
                            x2,
                            y2,
                            width_px,
                            color,
                            (255.0 * m.opacity.clamp(0.0, 1.0)) as u8,
                        );
                    } else {
                        draw_thick_line(
                            &mut img,
                            x1,
                            y1,
                            x2,
                            y2,
                            width_px,
                            color,
                            (255.0 * m.opacity.clamp(0.0, 1.0)) as u8,
                        );
                    }
                    let hs = *head_size as f32;
                    draw_arrow_head(
                        &mut img,
                        x1,
                        y1,
                        x2,
                        y2,
                        hs,
                        color,
                        (255.0 * m.opacity.clamp(0.0, 1.0)) as u8,
                    );
                }
                AnnotationKind::Mosaic { level } => {
                    let m = &ann.meta;
                    let block = mosaic_block_size(*level);
                    apply_mosaic(
                        &mut img,
                        &base_pixels,
                        m.x as i32,
                        m.y as i32,
                        m.w as i32,
                        m.h as i32,
                        block,
                    );
                }
                AnnotationKind::Freehand { points, smoothing } => {
                    if points.len() < 2 {
                        continue;
                    }
                    let m = &ann.meta;
                    let (r, g, b) = m
                        .stroke_color
                        .as_ref()
                        .and_then(|c| parse_hex_color(c))
                        .unwrap_or((255, 255, 255));
                    let a = (255.0 * m.opacity.clamp(0.0, 1.0)) as u8;
                    let mut pts: Vec<(f32, f32)> = points.clone();
                    // Chaikin smoothing passes based on smoothing factor (0..1) -> up to 3 passes
                    let passes = if *smoothing <= 0.0 {
                        0
                    } else if *smoothing < 0.34 {
                        1
                    } else if *smoothing < 0.67 {
                        2
                    } else {
                        3
                    };
                    for _ in 0..passes {
                        pts = chaikin(&pts);
                        if pts.len() > 4096 {
                            break;
                        }
                    }
                    let width_px = m.stroke_width.unwrap_or(2.0).max(1.0) as i32;
                    // 依据 stroke_color + stroke_width; 如果设置 dashed 则依据每段长度绘制
                    for w in pts.windows(2) {
                        let (x0, y0) = (w[0].0 as i32, w[0].1 as i32);
                        let (x1, y1) = (w[1].0 as i32, w[1].1 as i32);
                        // 目前 Freehand 仅支持实线 (后续可基于 meta / 额外字段扩展虚线)
                        draw_thick_line(&mut img, x0, y0, x1, y1, width_px, (r, g, b), a);
                    }
                }
                AnnotationKind::Text {
                    content,
                    font_family: _,
                    font_size,
                } => {
                    // 初版占位实现：按固定宽度网格填充字符块，后续引入 fontdue 栅格真正字形
                    let m = &ann.meta;
                    let opacity = m.opacity.clamp(0.0, 1.0);
                    let (r, g, b) = m
                        .fill_color
                        .as_ref()
                        .and_then(|c| parse_hex_color(c))
                        .or_else(|| m.stroke_color.as_ref().and_then(|c| parse_hex_color(c)))
                        .unwrap_or((255, 255, 255));
                    let cell_w = ((*font_size as f32) * 0.6).ceil() as i32; // 粗略宽度
                    let cell_h = *font_size as i32;
                    for (i, _ch) in content.chars().enumerate() {
                        let x = m.x as i32 + i as i32 * cell_w;
                        let y = m.y as i32;
                        blend_fill_rect(
                            &mut img,
                            x,
                            y,
                            cell_w.max(1),
                            cell_h.max(1),
                            r,
                            g,
                            b,
                            (255.0 * opacity) as u8,
                        );
                    }
                }
            }
        }
        img
    }
}

fn parse_hex_color(s: &str) -> Option<(u8, u8, u8)> {
    let ss = s.strip_prefix('#').unwrap_or(s);
    if ss.len() == 6 {
        u32::from_str_radix(ss, 16).ok().map(|v| {
            (
                ((v >> 16) & 0xFF) as u8,
                ((v >> 8) & 0xFF) as u8,
                (v & 0xFF) as u8,
            )
        })
    } else {
        None
    }
}

#[allow(clippy::too_many_arguments)]
fn blend_fill_rect(img: &mut Image, x: i32, y: i32, w: i32, h: i32, r: u8, g: u8, b: u8, a: u8) {
    let (w0, h0) = (img.width as i32, img.height as i32);
    let x2 = (x + w).min(w0);
    let y2 = (y + h).min(h0);
    let xs = x.max(0);
    let ys = y.max(0);
    for yy in ys..y2 {
        for xx in xs..x2 {
            let i = ((yy as u32 * img.width + xx as u32) * 4) as usize;
            let dst = &mut img.pixels[i..i + 4];
            let src_a = a as f32 / 255.0;
            let dst_a = dst[3] as f32 / 255.0;
            let out_a = src_a + dst_a * (1.0 - src_a);
            if out_a > 0.0 {
                dst[0] =
                    (((r as f32 * src_a) + (dst[0] as f32 * dst_a * (1.0 - src_a))) / out_a) as u8;
                dst[1] =
                    (((g as f32 * src_a) + (dst[1] as f32 * dst_a * (1.0 - src_a))) / out_a) as u8;
                dst[2] =
                    (((b as f32 * src_a) + (dst[2] as f32 * dst_a * (1.0 - src_a))) / out_a) as u8;
                dst[3] = (out_a * 255.0) as u8;
            } else {
                dst[0] = r;
                dst[1] = g;
                dst[2] = b;
                dst[3] = a;
            }
        }
    }
}

// 添加：Blend 模式与辅助函数
#[derive(Copy, Clone)]
enum Blend {
    Multiply,
    Screen,
}

fn blend_pixel_mode(dst: &mut [u8], sr: (u8, u8, u8, u8), mode: Blend) {
    let (sr, sg, sb, sa_f) = (sr.0 as f32, sr.1 as f32, sr.2 as f32, sr.3 as f32 / 255.0);
    if sa_f == 0.0 {
        return;
    }
    let (dr, dg, db, da_f) = (
        dst[0] as f32,
        dst[1] as f32,
        dst[2] as f32,
        dst[3] as f32 / 255.0,
    );
    let (mut br, mut bg, mut bb) = match mode {
        Blend::Multiply => ((sr * dr) / 255.0, (sg * dg) / 255.0, (sb * db) / 255.0),
        Blend::Screen => (
            255.0 - (255.0 - sr) * (255.0 - dr) / 255.0,
            255.0 - (255.0 - sg) * (255.0 - dg) / 255.0,
            255.0 - (255.0 - sb) * (255.0 - db) / 255.0,
        ),
    };
    let out_a = sa_f + da_f * (1.0 - sa_f);
    if out_a > 0.0 {
        br = (br * sa_f + dr * da_f * (1.0 - sa_f)) / out_a;
        bg = (bg * sa_f + dg * da_f * (1.0 - sa_f)) / out_a;
        bb = (bb * sa_f + db * da_f * (1.0 - sa_f)) / out_a;
    }
    dst[0] = br as u8;
    dst[1] = bg as u8;
    dst[2] = bb as u8;
    dst[3] = (out_a * 255.0) as u8;
}

#[allow(clippy::too_many_arguments)]
fn stroke_rect(
    img: &mut Image,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    th: f32,
    r: u8,
    g: u8,
    b: u8,
    a: u8,
) {
    if th <= 0.0 {
        return;
    }
    let t = th.ceil() as i32;
    // 四条边: top, bottom, left, right
    blend_fill_rect(img, x, y, w, t, r, g, b, a);
    blend_fill_rect(img, x, y + h - t, w, t, r, g, b, a);
    blend_fill_rect(img, x, y, t, h, r, g, b, a);
    blend_fill_rect(img, x + w - t, y, t, h, r, g, b, a);
}

#[allow(clippy::too_many_arguments)]
fn highlight_rect(
    img: &mut Image,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    r: u8,
    g: u8,
    b: u8,
    a: u8,
    mode: Blend,
) {
    let (w0, h0) = (img.width as i32, img.height as i32);
    let x2 = (x + w).min(w0);
    let y2 = (y + h).min(h0);
    let xs = x.max(0);
    let ys = y.max(0);
    for yy in ys..y2 {
        for xx in xs..x2 {
            let i = ((yy as u32 * img.width + xx as u32) * 4) as usize;
            blend_pixel_mode(&mut img.pixels[i..i + 4], (r, g, b, a), mode);
        }
    }
}

fn mosaic_block_size(level: u8) -> i32 {
    match level {
        0 | 1 => 6,
        2 => 12,
        3 => 18,
        n => 6 + (n as i32 - 1) * 6,
    }
}

/// 应用马赛克效果（并行版本）
///
/// 性能优化：使用 rayon 并行处理马赛克块
/// - 每个块独立计算平均值
/// - 避免数据竞争（每个块写入不同的像素区域）
fn apply_mosaic(img: &mut Image, base: &[u8], x: i32, y: i32, w: i32, h: i32, block: i32) {
    use rayon::prelude::*;

    let (w0, h0) = (img.width as i32, img.height as i32);
    let x2 = (x + w).min(w0);
    let y2 = (y + h).min(h0);
    let xs = x.max(0);
    let ys = y.max(0);

    // 收集所有需要处理的块坐标
    let blocks: Vec<(i32, i32)> = (ys..y2)
        .step_by(block as usize)
        .flat_map(|by| (xs..x2).step_by(block as usize).map(move |bx| (bx, by)))
        .collect();

    // 并行计算每个块的平均颜色
    let block_colors: Vec<_> = blocks
        .par_iter()
        .filter_map(|&(bx, by)| {
            let bw = (bx + block).min(x2) - bx;
            let bh = (by + block).min(y2) - by;

            // 计算平均
            let mut acc_r = 0u32;
            let mut acc_g = 0u32;
            let mut acc_b = 0u32;
            let mut acc_a = 0u32;
            let mut count = 0u32;

            for yy in by..by + bh {
                for xx in bx..bx + bw {
                    let i = ((yy as u32 * img.width + xx as u32) * 4) as usize;
                    if i + 3 < base.len() {
                        acc_r += base[i] as u32;
                        acc_g += base[i + 1] as u32;
                        acc_b += base[i + 2] as u32;
                        acc_a += base[i + 3] as u32;
                        count += 1;
                    }
                }
            }

            if count == 0 {
                return None;
            }

            let r = (acc_r / count) as u8;
            let g = (acc_g / count) as u8;
            let b = (acc_b / count) as u8;
            let a = (acc_a / count) as u8;

            Some((bx, by, bw, bh, r, g, b, a))
        })
        .collect();

    // 串行应用颜色到图像（避免数据竞争）
    // 由于每个块写入不同区域，这里也可以并行，但为了简单起见先串行
    for (bx, by, bw, bh, r, g, b, a) in block_colors {
        for yy in by..by + bh {
            for xx in bx..bx + bw {
                let i = ((yy as u32 * img.width + xx as u32) * 4) as usize;
                if i + 3 < img.pixels.len() {
                    img.pixels[i] = r;
                    img.pixels[i + 1] = g;
                    img.pixels[i + 2] = b;
                    img.pixels[i + 3] = a;
                }
            }
        }
    }
}

fn chaikin(pts: &[(f32, f32)]) -> Vec<(f32, f32)> {
    if pts.len() < 2 {
        return pts.to_vec();
    }
    let mut out = Vec::with_capacity(pts.len() * 2);
    out.push(pts[0]);
    for w in pts.windows(2) {
        let (p0, p1) = (w[0], w[1]);
        let q = (0.75 * p0.0 + 0.25 * p1.0, 0.75 * p0.1 + 0.25 * p1.1);
        let r = (0.25 * p0.0 + 0.75 * p1.0, 0.25 * p0.1 + 0.75 * p1.1);
        out.push(q);
        out.push(r);
    }
    // 安全：前面已检查 pts.len() >= 2，所以 last() 必定有值
    if let Some(&last_pt) = pts.last() {
        out.push(last_pt);
    }
    out
}

pub trait ExportEncoder {
    fn encode_png(&self, img: &Image) -> anyhow::Result<Vec<u8>>;
    fn encode_jpeg(&self, _img: &Image, _quality: u8) -> anyhow::Result<Vec<u8>> {
        anyhow::bail!("jpeg not implemented")
    }
}

pub struct PngEncoder;
impl ExportEncoder for PngEncoder {
    fn encode_png(&self, img: &Image) -> anyhow::Result<Vec<u8>> {
        let mut buf = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut buf, img.width, img.height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_compression(png::Compression::Fast);
            encoder.set_filter(png::FilterType::Paeth);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header()?;
            writer.write_image_data(&img.pixels)?;
        }
        Ok(buf)
    }
    fn encode_jpeg(&self, img: &Image, quality: u8) -> anyhow::Result<Vec<u8>> {
        let mut rgb = Vec::with_capacity((img.width * img.height * 3) as usize);
        for px in img.pixels.chunks_exact(4) {
            rgb.extend_from_slice(&[px[0], px[1], px[2]]);
        }
        let mut out = Vec::new();
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality);
        use image::ExtendedColorType;
        encoder.encode(&rgb, img.width, img.height, ExtendedColorType::Rgb8)?;
        Ok(out)
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_thick_line(
    img: &mut Image,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    th: i32,
    (r, g, b): (u8, u8, u8),
    a: u8,
) {
    // Bresenham 基础实现, 对每个像素扩展一个圆形近似的方形厚度
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut x = x0;
    let mut y = y0;
    loop {
        for oy in -th / 2..=th / 2 {
            for ox in -th / 2..=th / 2 {
                blend_fill_rect(img, x + ox, y + oy, 1, 1, r, g, b, a);
            }
        }
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_arrow_head(
    img: &mut Image,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    size: f32,
    (r, g, b): (u8, u8, u8),
    a: u8,
) {
    let dx = (x1 - x0) as f32;
    let dy = (y1 - y0) as f32;
    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    let ux = dx / len;
    let uy = dy / len;
    // 基础等腰三角: 尾点为终点 (x1,y1)
    let bx = x1 as f32 - ux * size;
    let by = y1 as f32 - uy * size;
    let perp_x = -uy;
    let perp_y = ux;
    let w = size * 0.5;
    let p1 = (x1 as f32, y1 as f32);
    let p2 = (bx + perp_x * w, by + perp_y * w);
    let p3 = (bx - perp_x * w, by - perp_y * w);
    fill_triangle(img, p1, p2, p3, r, g, b, a);
}

#[allow(clippy::too_many_arguments)]
fn fill_triangle(
    img: &mut Image,
    p1: (f32, f32),
    p2: (f32, f32),
    p3: (f32, f32),
    r: u8,
    g: u8,
    b: u8,
    a: u8,
) {
    let (x1, y1) = p1;
    let (x2, y2) = p2;
    let (x3, y3) = p3;
    let min_x = x1.min(x2).min(x3).floor() as i32;
    let max_x = x1.max(x2).max(x3).ceil() as i32;
    let min_y = y1.min(y2).min(y3).floor() as i32;
    let max_y = y1.max(y2).max(y3).ceil() as i32;
    let area = (x2 - x1) * (y3 - y1) - (x3 - x1) * (y2 - y1);
    if area.abs() < f32::EPSILON {
        return;
    }
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let w0 = (x2 - x1) * (py - y1) - (y2 - y1) * (px - x1);
            let w1 = (x3 - x2) * (py - y2) - (y3 - y2) * (px - x2);
            let w2 = (x1 - x3) * (py - y3) - (y1 - y3) * (px - x3);
            if (w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0) || (w0 <= 0.0 && w1 <= 0.0 && w2 <= 0.0) {
                blend_fill_rect(img, x, y, 1, 1, r, g, b, a);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_dashed_line(
    img: &mut Image,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    th: i32,
    (r, g, b): (u8, u8, u8),
    a: u8,
) {
    // 基于总长度拆分 dash(开) 与 gap(关)，dash_len = 4*th, gap_len = 2*th
    let dx = (x1 - x0) as f32;
    let dy = (y1 - y0) as f32;
    let len = (dx * dx + dy * dy).sqrt();
    if len == 0.0 {
        return;
    }
    let ux = dx / len;
    let uy = dy / len;
    let dash_len = (4 * th).max(2) as f32;
    let gap_len = (2 * th).max(1) as f32;
    let mut _dist = 0.0;
    let mut cur = 0.0;
    while cur < len {
        // 绘制 dash 段
        let seg = (dash_len).min(len - cur);
        // 将 seg 离散为若干像素段，使用插值调用 draw_thick_line 分段 (近似)
        let steps = (seg.max(1.0)) as i32;
        let start_x = x0 as f32 + ux * cur;
        let start_y = y0 as f32 + uy * cur;
        for s in 0..steps {
            let t0 = s as f32 / steps as f32;
            let t1 = (s + 1) as f32 / steps as f32;
            if t0 >= 1.0 {
                break;
            }
            let sx = start_x + ux * seg * t0;
            let sy = start_y + uy * seg * t0;
            let ex = start_x + ux * seg * t1;
            let ey = start_y + uy * seg * t1;
            draw_thick_line(
                img,
                sx.round() as i32,
                sy.round() as i32,
                ex.round() as i32,
                ey.round() as i32,
                th,
                (r, g, b),
                a,
            );
        }
        cur += seg + gap_len;
        _dist += seg + gap_len;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_rect(x: f32, y: f32, w: f32, h: f32, color: &str, opacity: f32, z: i32) -> Annotation {
        // 使用 v7 时间序列 UUID (测试场景无需稳定种子)
        let ts = uuid::Timestamp::now(uuid::NoContext);
        Annotation {
            meta: screenshot_core::AnnotationMeta {
                id: Uuid::new_v7(ts),
                x,
                y,
                w,
                h,
                rotation: 0,
                opacity,
                stroke_color: None,
                fill_color: Some(color.to_string()),
                stroke_width: None,
                z,
                locked: false,
                created_at: Utc::now(),
            },
            kind: AnnotationKind::Rect { corner_radius: 0 },
        }
    }

    fn dummy_frame(w: u32, h: u32) -> Frame {
        use std::sync::Arc;
        Frame {
            width: w,
            height: h,
            pixel_format: PixelFormat::Rgba8,
            bytes: Arc::from(vec![0u8; (w * h * 4) as usize].into_boxed_slice()),
        }
    }

    #[test]
    fn test_image_fill_rect_and_png() {
        let mut img = Image::new(8, 8);
        img.fill_rect(2, 2, 3, 3, 255, 0, 0, 255);
        // 统计红色像素块数量
        let mut count = 0;
        for p in img.pixels.chunks_exact(4) {
            if p == [255, 0, 0, 255] {
                count += 1;
            }
        }
        assert_eq!(count, 9); // 3x3
        let enc = PngEncoder;
        let data = enc.encode_png(&img).unwrap();
        // PNG signature
        assert_eq!(&data[..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn test_rect_render_order_and_alpha() {
        let r = SimpleRenderer;
        let a1 = make_rect(0.0, 0.0, 4.0, 4.0, "#FF0000", 1.0, 0); // red base
        let a2 = make_rect(2.0, 2.0, 4.0, 4.0, "#0000FF", 0.5, 1); // semi blue on top
        let frame = dummy_frame(6, 6);
        let img = r.render(&frame, &[a1, a2]);
        // Pixel (1,1) only red
        let idx = ((6 + 1) * 4) as usize;
        assert_eq!(&img.pixels[idx..idx + 4], [255, 0, 0, 255]);
        // Pixel (3,3) overlap blended
        let idx2 = ((3 * 6 + 3) * 4) as usize;
        let px = &img.pixels[idx2..idx2 + 4];
        assert!(px[0] < 255 && px[2] > 0 && px[3] > 255 / 2); // blended, alpha > 127
    }

    #[test]
    fn test_rect_stroke() {
        let r = SimpleRenderer;
        // base transparent
        let mut rect = make_rect(1.0, 1.0, 4.0, 4.0, "#00FF00", 0.4, 0);
        rect.meta.stroke_color = Some("#FF0000".into());
        rect.meta.stroke_width = Some(1.0);
        rect.meta.opacity = 1.0;
        let frame = dummy_frame(8, 8);
        let img = r.render(&frame, &[rect]);
        // Corner outside stroke not drawn
        assert_eq!(&img.pixels[0..4], [0, 0, 0, 0]);
        // Stroke pixel at (1,1) should be red (allow some blend with fill interior at (2,2))
        let idx = ((8 + 1) * 4) as usize;
        let p = &img.pixels[idx..idx + 4];
        assert!(p[0] > 180 && p[1] < 80); // red dominance
    }

    #[test]
    fn test_highlight_modes() {
        // Prepare background red rect then highlight with screen (blue) to lighten
        let r = SimpleRenderer;
        let base = make_rect(0.0, 0.0, 4.0, 4.0, "#800000", 1.0, 0);
        let mut hl_meta = base.meta.clone();
        hl_meta.x = 0.0;
        hl_meta.y = 0.0;
        hl_meta.w = 4.0;
        hl_meta.h = 4.0;
        hl_meta.fill_color = Some("#0000FF".into());
        hl_meta.opacity = 0.5;
        hl_meta.z = 1;
        let highlight = Annotation {
            meta: hl_meta,
            kind: AnnotationKind::Highlight {
                mode: BlendMode::Screen,
            },
        };
        let frame = dummy_frame(4, 4);
        let img = r.render(&frame, &[base, highlight]);
        let center = &img.pixels[((4 + 1) * 4) as usize..((4 + 1) * 4 + 4) as usize];
        // Expect both red and blue channels > 0
        assert!(center[0] > 60 && center[2] > 60);
    }

    #[test]
    fn test_arrow_render() {
        use chrono::Utc;
        use screenshot_core::{AnnotationKind, AnnotationMeta};
        use uuid::Uuid;
        let ts = uuid::Timestamp::now(uuid::NoContext);
        let meta = AnnotationMeta {
            id: Uuid::new_v7(ts),
            x: 0.0,
            y: 0.0,
            w: 10.0,
            h: 10.0,
            rotation: 0,
            opacity: 1.0,
            stroke_color: Some("#00FF00".into()),
            fill_color: None,
            stroke_width: Some(2.0),
            z: 0,
            locked: false,
            created_at: Utc::now(),
        };
        let arrow = Annotation {
            meta,
            kind: AnnotationKind::Arrow {
                head_size: 8,
                line_style: screenshot_core::LineStyle::Solid,
            },
        };
        let r = SimpleRenderer;
        let frame = dummy_frame(12, 12);
        let img = r.render(&frame, &[arrow]);
        // 检查对角线上至少有若干绿色像素
        let mut diag = 0;
        for i in 0..12 {
            let idx = ((i * 12 + i) * 4) as usize;
            let p = &img.pixels[idx..idx + 4];
            if p[1] > 150 && p[0] < 50 {
                diag += 1;
            }
        }
        assert!(diag >= 5, "diag green pixels insufficient: {}", diag);
        // 终点附近应更粗(箭头头部)，检查终点像素块
        let end_idx = ((10 * 12 + 10) * 4) as usize;
        assert!(img.pixels[end_idx + 1] > 150);
    }

    #[test]
    fn test_mosaic_blocks() {
        use std::sync::Arc;
        let mut buf = vec![0u8; 16 * 16 * 4];
        for y in 0..16 {
            for x in 0..16 {
                let i = ((y * 16 + x) * 4) as usize;
                buf[i] = (x * 16) as u8;
                buf[i + 1] = (y * 16) as u8;
                buf[i + 2] = 128;
                buf[i + 3] = 255;
            }
        }
        let base = Frame {
            width: 16,
            height: 16,
            pixel_format: PixelFormat::Rgba8,
            bytes: Arc::from(buf.into_boxed_slice()),
        };
        let r = SimpleRenderer;
        use uuid::Uuid;
        let ts = uuid::Timestamp::now(uuid::NoContext);
        let meta = screenshot_core::AnnotationMeta {
            id: Uuid::new_v7(ts),
            x: 0.0,
            y: 0.0,
            w: 16.0,
            h: 16.0,
            rotation: 0,
            opacity: 1.0,
            stroke_color: None,
            fill_color: None,
            stroke_width: None,
            z: 0,
            locked: false,
            created_at: Utc::now(),
        };
        let mosaic = Annotation {
            meta,
            kind: AnnotationKind::Mosaic { level: 1 },
        };
        let img = r.render(&base, &[mosaic]);
        // Sample two pixels inside first block should be identical after mosaic
        let p1 = &img.pixels[0..4];
        let p2 = &img.pixels[4..8];
        assert_eq!(p1, p2);
        // Pixel far away should differ (different block)
        let p_far = &img.pixels[((10 * 16 + 10) * 4) as usize..((10 * 16 + 10) * 4 + 4) as usize];
        assert_ne!(p1, p_far);
    }

    #[test]
    fn test_text_placeholder_render() {
        use chrono::Utc;
        use screenshot_core::{AnnotationKind, AnnotationMeta};
        use uuid::Uuid;
        let ts = uuid::Timestamp::now(uuid::NoContext);
        let meta = AnnotationMeta {
            id: Uuid::new_v7(ts),
            x: 1.0,
            y: 2.0,
            w: 100.0,
            h: 20.0,
            rotation: 0,
            opacity: 1.0,
            stroke_color: None,
            fill_color: Some("#112233".into()),
            stroke_width: None,
            z: 0,
            locked: false,
            created_at: Utc::now(),
        };
        let text = Annotation {
            meta,
            kind: AnnotationKind::Text {
                content: "Hi".into(),
                font_family: "system".into(),
                font_size: 12,
            },
        };
        let r = SimpleRenderer;
        let frame = dummy_frame(64, 32);
        let img = r.render(&frame, &[text]);
        // 检查指定区域内存在非透明像素
        let mut colored = 0;
        for y in 2..14 {
            for x in 1..30 {
                let idx = ((y * 64 + x) * 4) as usize;
                if img.pixels[idx + 3] > 0 {
                    colored += 1;
                }
            }
        }
        assert!(
            colored > 50,
            "expected placeholder text blocks, got {}",
            colored
        );
    }

    #[test]
    fn test_freehand_smoothing() {
        use chrono::Utc;
        use screenshot_core::{AnnotationKind, AnnotationMeta};
        use uuid::Uuid;
        // Zigzag points
        let pts: Vec<(f32, f32)> = (0..20)
            .map(|i| (i as f32 * 2.0, if i % 2 == 0 { 0.0 } else { 10.0 }))
            .collect();
        let ts = uuid::Timestamp::now(uuid::NoContext);
        let mut meta = AnnotationMeta {
            id: Uuid::new_v7(ts),
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
            rotation: 0,
            opacity: 1.0,
            stroke_color: Some("#FF0000".into()),
            fill_color: None,
            stroke_width: Some(2.0),
            z: 0,
            locked: false,
            created_at: Utc::now(),
        };
        let freehand = Annotation {
            meta: meta.clone(),
            kind: AnnotationKind::Freehand {
                points: pts.clone(),
                smoothing: 1.0,
            },
        };
        let freehand_raw = Annotation {
            meta: {
                meta.z = 1;
                meta.clone()
            },
            kind: AnnotationKind::Freehand {
                points: pts.clone(),
                smoothing: 0.0,
            },
        };
        let r = SimpleRenderer;
        let frame = dummy_frame(80, 40);
        let img = r.render(&frame, &[freehand_raw, freehand]);
        // Expect smoothed line fills some mid y pixels around y=5 (blended) unlike purely 0 or 10
        let mut mid = 0;
        for y in 3..8 {
            for x in 0..80 {
                let idx = ((y * 80 + x) * 4) as usize;
                if img.pixels[idx + 3] > 0 {
                    mid += 1;
                }
            }
        }
        assert!(mid > 30, "expected smoothed mid coverage >30 got {}", mid);
    }

    #[test]
    fn test_dashed_arrow() {
        use chrono::Utc;
        use screenshot_core::{AnnotationKind, AnnotationMeta, LineStyle};
        use uuid::Uuid;
        let ts = uuid::Timestamp::now(uuid::NoContext);
        let meta_solid = AnnotationMeta {
            id: Uuid::new_v7(ts),
            x: 0.0,
            y: 0.0,
            w: 50.0,
            h: 0.0,
            rotation: 0,
            opacity: 1.0,
            stroke_color: Some("#00FF00".into()),
            fill_color: None,
            stroke_width: Some(2.0),
            z: 0,
            locked: false,
            created_at: Utc::now(),
        };
        let ts2 = uuid::Timestamp::now(uuid::NoContext);
        let meta_dash = AnnotationMeta {
            id: Uuid::new_v7(ts2),
            x: 0.0,
            y: 5.0,
            w: 50.0,
            h: 0.0,
            rotation: 0,
            opacity: 1.0,
            stroke_color: Some("#00FF00".into()),
            fill_color: None,
            stroke_width: Some(2.0),
            z: 0,
            locked: false,
            created_at: Utc::now(),
        };
        let solid = Annotation {
            meta: meta_solid,
            kind: AnnotationKind::Arrow {
                head_size: 8,
                line_style: LineStyle::Solid,
            },
        };
        let dashed = Annotation {
            meta: meta_dash,
            kind: AnnotationKind::Arrow {
                head_size: 8,
                line_style: LineStyle::Dashed,
            },
        };
        let r = SimpleRenderer;
        let frame = dummy_frame(64, 16);
        let img = r.render(&frame, &[solid, dashed]);
        // Count green pixels in solid row y=0 and dashed row y=5
        let mut solid_count = 0;
        let mut dashed_count = 0;
        for x in 0..64 {
            let idx = (x * 4) as usize;
            let p = &img.pixels[idx..idx + 4];
            if p[1] > 150 {
                solid_count += 1;
            }
            let idx2 = ((5 * 64 + x) * 4) as usize;
            let p2 = &img.pixels[idx2..idx2 + 4];
            if p2[1] > 150 {
                dashed_count += 1;
            }
        }
        assert!(
            solid_count > dashed_count,
            "solid {} should be > dashed {}",
            solid_count,
            dashed_count
        );
        assert!(
            dashed_count > 5,
            "dashed should still have some pixels {}",
            dashed_count
        );
    }

    #[test]
    fn test_jpeg_encode() {
        let mut img = Image::new(16, 16);
        img.fill_rect(0, 0, 16, 16, 10, 20, 30, 255);
        let enc = PngEncoder; // same encoder implements jpeg
        let jpeg = enc.encode_jpeg(&img, 80).unwrap();
        // JPEG SOI marker 0xFFD8 and some length > 100 bytes for even tiny image
        assert!(jpeg.len() > 50, "jpeg too small {}", jpeg.len());
        assert_eq!(&jpeg[0..2], &[0xFF, 0xD8]);
    }
}
