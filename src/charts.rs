//! PNG charts for NVIDIA revenue. All financial values are in US$ millions.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use plotters::{
    coord::{Shift, types::RangedCoordf64},
    prelude::*,
    style::text_anchor::{HPos, Pos, VPos},
};

use crate::RevenueRow;

const LABELS: [&str; 6] = [
    "Data Centre",
    "Gaming",
    "Professional Visualisation",
    "Automotive",
    "OEM & Other",
    "Total",
];
const COLOURS: [RGBColor; 6] = [
    RGBColor(118, 185, 0),
    RGBColor(30, 136, 229),
    RGBColor(255, 167, 38),
    RGBColor(171, 71, 188),
    RGBColor(120, 144, 156),
    RGBColor(46, 46, 46),
];
const GOLD: RGBColor = RGBColor(180, 130, 0);
const GRID: RGBColor = RGBColor(225, 230, 232);
type Canvas<'a> = DrawingArea<BitMapBackend<'a>, Shift>;
type Chart<'a, 'b> =
    ChartContext<'a, BitMapBackend<'b>, Cartesian2d<RangedCoordf64, RangedCoordf64>>;

/// Generate the same nine chart views as the original application.
///
/// Y/Y requires an observation of the same fiscal quarter in the prior year.
/// A zero baseline has no defined indexed growth and is shown as a gap.
pub fn generate(rows: &[RevenueRow], output_dir: &Path) -> Result<Vec<PathBuf>> {
    if rows.is_empty() {
        bail!("cannot generate charts without revenue data");
    }
    fs::create_dir_all(output_dir)
        .with_context(|| format!("create chart directory {}", output_dir.display()))?;
    let mut outputs = Vec::new();
    let path = output_dir.join("nvidia-revenue-trend.png");
    revenue_trend(rows, &path)?;
    outputs.push(path);
    let path = output_dir.join("market_share_chart.png");
    market_share(rows, &path)?;
    outputs.push(path);
    let path = output_dir.join("stacked_area_chart.png");
    stacked_area(rows, &path)?;
    outputs.push(path);
    let path = output_dir.join("segment_trends.png");
    let data = rows
        .iter()
        .map(|row| row.values().map(Some))
        .collect::<Vec<_>>();
    line_chart(
        rows,
        &data,
        5,
        "NVIDIA Quarterly Revenue: Segment Trends",
        "Revenue (US$ millions)",
        None,
        "",
        &path,
    )?;
    outputs.push(path);
    let path = output_dir.join("growth_rate_qoq.png");
    let (quarters, data) = growth_series(rows, 1);
    grouped_bars(
        &quarters,
        &data,
        "NVIDIA Quarterly Revenue: Q/Q Growth Rate Comparison",
        "Growth rate (%)",
        false,
        &path,
    )?;
    outputs.push(path);
    let (quarters, data) = growth_series(rows, 4);
    if !quarters.is_empty() {
        let path = output_dir.join("growth_rate_yoy.png");
        grouped_bars(
            &quarters,
            &data,
            "NVIDIA Quarterly Revenue: Y/Y Growth Rate Comparison",
            "Growth rate (%)",
            false,
            &path,
        )?;
        outputs.push(path);
    } else {
        // Do not leave a previous run's Y/Y image beside newer charts.
        let stale = output_dir.join("growth_rate_yoy.png");
        if stale.exists() {
            fs::remove_file(stale)?;
        }
    }
    let path = output_dir.join("cagr_chart.png");
    let data = cagr_series(rows);
    line_chart(
        rows,
        &data,
        6,
        &format!(
            "NVIDIA Revenue: Compound Annual Growth Rate from {}",
            rows[0].quarter
        ),
        "CAGR (%)",
        Some(0.0),
        "Annualized over elapsed fiscal quarters; undefined values are omitted.",
        &path,
    )?;
    outputs.push(path);
    let path = output_dir.join("revenue_contribution.png");
    let (quarters, data) = contribution_series(rows);
    grouped_bars(
        &quarters,
        &data,
        "NVIDIA Revenue: Segment Contribution to Total Growth",
        "Contribution to total growth (%)",
        true,
        &path,
    )?;
    outputs.push(path);
    let path = output_dir.join("normalized_growth.png");
    let data = normalized_series(rows);
    line_chart(
        rows,
        &data,
        6,
        &format!("NVIDIA Revenue: Normalized Growth from {}", rows[0].quarter),
        "Indexed revenue (baseline = 100)",
        Some(100.0),
        "Segments with a zero baseline are omitted because their index is undefined.",
        &path,
    )?;
    outputs.push(path);
    Ok(outputs)
}

fn period(row: &RevenueRow) -> i64 {
    i64::from(row.fiscal_year) * 4 + i64::from(row.quarter_number)
}

fn previous(rows: &[RevenueRow], index: usize, lag: i64) -> Option<&RevenueRow> {
    let target = period(&rows[index]) - lag;
    rows[..index].iter().rev().find(|row| period(row) == target)
}

fn growth_series(rows: &[RevenueRow], lag: i64) -> (Vec<String>, Vec<[f64; 5]>) {
    let mut quarters = Vec::new();
    let mut data = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        let prior = previous(rows, index, lag);
        if lag == 4 && prior.is_none() {
            continue;
        }
        quarters.push(row.quarter.clone());
        let values = row.values();
        data.push(std::array::from_fn(|segment| {
            prior.map_or(0.0, |prior| {
                crate::growth_rate(values[segment], prior.values()[segment])
            })
        }));
    }
    (quarters, data)
}

fn contribution_series(rows: &[RevenueRow]) -> (Vec<String>, Vec<[f64; 5]>) {
    let mut quarters = Vec::new();
    let mut data = Vec::new();
    for (index, row) in rows.iter().enumerate().skip(1) {
        let Some(prior) = previous(rows, index, 1) else {
            continue;
        };
        let values = row.values();
        let old = prior.values();
        let change = values[5] - old[5];
        quarters.push(row.quarter.clone());
        data.push(std::array::from_fn(|segment| {
            if change == 0.0 {
                0.0
            } else {
                (values[segment] - old[segment]) / change * 100.0
            }
        }));
    }
    (quarters, data)
}

fn cagr_series(rows: &[RevenueRow]) -> Vec<[Option<f64>; 6]> {
    let baseline = rows[0].values();
    rows.iter()
        .map(|row| {
            let values = row.values();
            let elapsed = period(row) - period(&rows[0]);
            std::array::from_fn(|segment| {
                if elapsed == 0 || baseline[segment] <= 0.0 {
                    Some(0.0)
                } else if values[segment] < 0.0 || elapsed < 0 {
                    None
                } else {
                    let cagr = ((values[segment] / baseline[segment]).powf(4.0 / elapsed as f64)
                        - 1.0)
                        * 100.0;
                    cagr.is_finite().then_some(cagr)
                }
            })
        })
        .collect()
}

fn normalized_series(rows: &[RevenueRow]) -> Vec<[Option<f64>; 6]> {
    let baseline = rows[0].values();
    rows.iter()
        .map(|row| {
            let values = row.values();
            std::array::from_fn(|segment| {
                (baseline[segment] != 0.0).then(|| values[segment] / baseline[segment] * 100.0)
            })
        })
        .collect()
}

fn percentages(row: &RevenueRow) -> [f64; 5] {
    let values = row.values();
    std::array::from_fn(|segment| {
        if values[5] == 0.0 {
            0.0
        } else {
            values[segment] / values[5] * 100.0
        }
    })
}

fn width(count: usize) -> u32 {
    // Labels stay legible on a short PDF window and large histories alike.
    (count.max(8).saturating_mul(180)).min(16_000) as u32
}

fn range(values: impl IntoIterator<Item = f64>, include: Option<f64>) -> std::ops::Range<f64> {
    let mut low: f64 = 0.0;
    let mut high: f64 = 0.0;
    for value in values
        .into_iter()
        .chain(include)
        .filter(|value| value.is_finite())
    {
        low = low.min(value);
        high = high.max(value);
    }
    let padding = ((high - low) * 0.12).max(1.0);
    (if low < 0.0 { low - padding } else { 0.0 })..(high + padding)
}

fn centred(size: u32, colour: &RGBColor) -> TextStyle<'static> {
    ("sans-serif", size)
        .into_font()
        .color(colour)
        .pos(Pos::new(HPos::Center, VPos::Center))
}

fn header<'a>(
    root: &Canvas<'a>,
    title: &str,
    segments: usize,
    extras: &[(&str, RGBColor)],
    note: &str,
) -> Result<Canvas<'a>> {
    let (heading, body) = root.split_vertically(135);
    let (w, _) = root.dim_in_pixel();
    heading.draw(&Text::new(
        title,
        (w as i32 / 2, 29),
        centred(28, &COLOURS[5]),
    ))?;
    let mut x = 28;
    let mut y = 70;
    let legends = LABELS[..segments]
        .iter()
        .zip(COLOURS[..segments].iter())
        .map(|(label, colour)| (*label, *colour))
        .chain(extras.iter().copied());
    for (label, colour) in legends {
        let item_width = label.len() as i32 * 11 + 62;
        if x + item_width > w as i32 - 24 {
            x = 28;
            y += 34;
        }
        heading.draw(&Rectangle::new(
            [(x, y - 7), (x + 22, y + 7)],
            colour.filled(),
        ))?;
        heading.draw(&Text::new(
            label,
            (x + 32, y),
            ("sans-serif", 20)
                .into_font()
                .color(&COLOURS[5])
                .pos(Pos::new(HPos::Left, VPos::Center)),
        ))?;
        x += item_width;
    }
    if note.is_empty() {
        Ok(body)
    } else {
        let (_, h) = body.dim_in_pixel();
        let (plot, footer) = body.split_vertically(h.saturating_sub(35));
        footer.draw(&Text::new(
            note,
            (w as i32 / 2, 16),
            centred(17, &COLOURS[5]),
        ))?;
        Ok(plot)
    }
}

fn make_chart<'a, 'b>(
    area: &'a Canvas<'b>,
    count: usize,
    y_range: std::ops::Range<f64>,
    y_label: &str,
    quarters: &[String],
) -> Result<Chart<'a, 'b>> {
    let min = y_range.start;
    let mut chart = ChartBuilder::on(area)
        .margin(20)
        .x_label_area_size(85)
        .y_label_area_size(110)
        .build_cartesian_2d(-0.6..count.max(1) as f64 - 0.4, y_range)?;
    chart
        .configure_mesh()
        .disable_x_mesh()
        .x_labels(0)
        .y_labels(8)
        .max_light_lines(0)
        .axis_desc_style(("sans-serif", 22))
        .label_style(("sans-serif", 19))
        .light_line_style(GRID)
        .bold_line_style(GRID)
        .x_desc("Quarter")
        .y_desc(y_label)
        .y_label_formatter(&|value| number(*value))
        .draw()?;
    let label_space = chart.plotting_area().dim_in_pixel().0 as usize / 125;
    let stride = quarters.len().div_ceil(label_space.max(1)).max(1);
    chart.draw_series(
        quarters
            .iter()
            .enumerate()
            .filter(|(index, _)| index % stride == 0 || *index + 1 == quarters.len())
            .map(|(index, quarter)| {
                EmptyElement::at((index as f64, min))
                    + Text::new(
                        quarter.clone(),
                        (0, 19),
                        ("sans-serif", 18)
                            .into_font()
                            .color(&COLOURS[5])
                            .pos(Pos::new(HPos::Center, VPos::Top)),
                    )
            }),
    )?;
    Ok(chart)
}

fn number(value: f64) -> String {
    let absolute = value.abs();
    if absolute >= 1_000_000.0 {
        format!("{:.1}m", value / 1_000_000.0)
    } else if absolute >= 10_000.0 {
        format!("{:.0}k", value / 1_000.0)
    } else if absolute >= 1_000.0 {
        format!("{:.1}k", value / 1_000.0)
    } else if absolute < 10.0 && value.fract().abs() > 0.01 {
        format!("{value:.1}")
    } else {
        format!("{value:.0}")
    }
}

fn line(
    chart: &mut Chart<'_, '_>,
    points: impl IntoIterator<Item = (f64, f64)>,
    colour: RGBColor,
    thickness: u32,
) -> Result<()> {
    let points: Vec<_> = points.into_iter().collect();
    chart.draw_series(LineSeries::new(
        points.iter().copied(),
        colour.stroke_width(thickness),
    ))?;
    chart.draw_series(
        points
            .iter()
            .copied()
            .map(|point| Circle::new(point, 5, colour.filled())),
    )?;
    Ok(())
}

fn revenue_trend(rows: &[RevenueRow], path: &Path) -> Result<()> {
    let root = BitMapBackend::new(path, (width(rows.len()), 920)).into_drawing_area();
    root.fill(&WHITE)?;
    let area = header(
        &root,
        "NVIDIA Quarterly Revenue: Segment Breakdown & Total Trend",
        5,
        &[
            ("Total Revenue", COLOURS[5]),
            ("Data Centre midpoint", GOLD),
        ],
        "Labels show Q/Q growth; the gold line sits at half Data Centre revenue.",
    )?;
    let data = rows.iter().map(RevenueRow::values).collect::<Vec<_>>();
    let extremes = data.iter().flat_map(|values| {
        [
            values[..5].iter().filter(|v| **v > 0.0).sum::<f64>(),
            values[..5].iter().filter(|v| **v < 0.0).sum::<f64>(),
            values[5],
        ]
    });
    let y_range = range(extremes, None);
    let quarters = rows
        .iter()
        .map(|row| row.quarter.clone())
        .collect::<Vec<_>>();
    let mut chart = make_chart(
        &area,
        rows.len(),
        y_range,
        "Revenue (US$ millions)",
        &quarters,
    )?;
    for (index, values) in data.iter().enumerate() {
        let mut positive = 0.0;
        let mut negative = 0.0;
        for (segment, value) in values.iter().copied().enumerate().take(5) {
            let base = if value < 0.0 {
                &mut negative
            } else {
                &mut positive
            };
            chart.draw_series(std::iter::once(Rectangle::new(
                [
                    (index as f64 - 0.33, *base),
                    (index as f64 + 0.33, *base + value),
                ],
                COLOURS[segment].mix(0.86).filled(),
            )))?;
            *base += value;
        }
    }
    line(
        &mut chart,
        data.iter()
            .enumerate()
            .map(|(i, values)| (i as f64, values[5])),
        COLOURS[5],
        4,
    )?;
    line(
        &mut chart,
        data.iter()
            .enumerate()
            .map(|(i, values)| (i as f64, values[0] / 2.0)),
        GOLD,
        3,
    )?;
    for (index, row) in rows.iter().enumerate() {
        let prior = previous(rows, index, 1);
        let total_growth = prior.map_or(0.0, |prior| {
            crate::growth_rate(row.total_revenue as f64, prior.total_revenue as f64)
        });
        let dc_growth = prior.map_or(0.0, |prior| {
            crate::growth_rate(row.data_center as f64, prior.data_center as f64)
        });
        chart.draw_series(std::iter::once(
            EmptyElement::at((index as f64, data[index][5]))
                + Text::new(
                    format!("{total_growth:+.1}%"),
                    (0, -22),
                    centred(18, &COLOURS[5]),
                ),
        ))?;
        chart.draw_series(std::iter::once(
            EmptyElement::at((index as f64, data[index][0] / 2.0))
                + Text::new(
                    format!("{dc_growth:+.1}%"),
                    (0, 22),
                    centred(18, &COLOURS[5]),
                ),
        ))?;
    }
    root.present()
        .with_context(|| format!("save {}", path.display()))
}

fn market_share(rows: &[RevenueRow], path: &Path) -> Result<()> {
    let columns = rows.len().min(4);
    let panel_rows = rows.len().div_ceil(columns);
    let dimensions = (
        (columns as u32 * 400).max(1_440),
        135 + panel_rows as u32 * 340,
    );
    let root = BitMapBackend::new(path, dimensions).into_drawing_area();
    root.fill(&WHITE)?;
    let area = header(
        &root,
        "NVIDIA Quarterly Revenue: Market Share by Segment",
        5,
        &[],
        "",
    )?;
    let panels = area.split_evenly((panel_rows, columns));
    for (row, panel) in rows.iter().zip(panels) {
        let (w, h) = panel.dim_in_pixel();
        let centre = (w as i32 / 2, h as i32 / 2 + 12);
        let radius = (w.min(h).saturating_sub(88) / 2) as f64;
        panel.draw(&Text::new(
            row.quarter.clone(),
            (w as i32 / 2, 29),
            centred(23, &COLOURS[5]),
        ))?;
        let values = row.values();
        let sum = values[..5].iter().map(|value| value.max(0.0)).sum::<f64>();
        if sum == 0.0 {
            panel.draw(&Circle::new(centre, radius as i32, GRID.stroke_width(2)))?;
            panel.draw(&Text::new("No revenue", centre, centred(19, &COLOURS[5])))?;
            continue;
        }
        let mut start = -std::f64::consts::FRAC_PI_2;
        for (segment, value) in values.iter().copied().enumerate().take(5) {
            let fraction = value.max(0.0) / sum;
            if fraction == 0.0 {
                continue;
            }
            let end = start + fraction * std::f64::consts::TAU;
            let steps = ((end - start) * radius).ceil().max(2.0) as usize;
            let mut polygon = vec![centre];
            polygon.extend((0..=steps).map(|step| {
                let angle = start + (end - start) * step as f64 / steps as f64;
                (
                    centre.0 + (radius * angle.cos()).round() as i32,
                    centre.1 + (radius * angle.sin()).round() as i32,
                )
            }));
            panel.draw(&Polygon::new(polygon, COLOURS[segment].filled()))?;
            if fraction > 0.05 {
                let angle = (start + end) / 2.0;
                let label = (
                    centre.0 + (radius * 0.78 * angle.cos()) as i32,
                    centre.1 + (radius * 0.78 * angle.sin()) as i32,
                );
                panel.draw(&Text::new(
                    format!("{:.1}%", fraction * 100.0),
                    label,
                    centred(18, &COLOURS[5]),
                ))?;
            }
            start = end;
        }
        panel.draw(&Circle::new(centre, (radius * 0.5) as i32, WHITE.filled()))?;
        panel.draw(&Text::new(
            number(values[5]),
            (centre.0, centre.1 - 9),
            centred(22, &COLOURS[5]),
        ))?;
        panel.draw(&Text::new(
            "US$ millions",
            (centre.0, centre.1 + 17),
            centred(15, &COLOURS[5]),
        ))?;
    }
    root.present()
        .with_context(|| format!("save {}", path.display()))
}

fn stacked_area(rows: &[RevenueRow], path: &Path) -> Result<()> {
    let root = BitMapBackend::new(path, (width(rows.len()), 880)).into_drawing_area();
    root.fill(&WHITE)?;
    let area = header(
        &root,
        "NVIDIA Revenue: Market Share Evolution (100% Stacked)",
        5,
        &[],
        "Percentages use reported total revenue; zero-total quarters have zero share.",
    )?;
    let quarters = rows
        .iter()
        .map(|row| row.quarter.clone())
        .collect::<Vec<_>>();
    let mut chart = make_chart(&area, rows.len(), 0.0..100.0, "Market share (%)", &quarters)?;
    let shares = rows.iter().map(percentages).collect::<Vec<_>>();
    let mut lower = vec![0.0; rows.len()];
    for (segment, colour) in COLOURS.iter().copied().enumerate().take(5) {
        let upper = lower
            .iter()
            .zip(&shares)
            .map(|(base, values)| base + values[segment])
            .collect::<Vec<_>>();
        if rows.len() == 1 {
            chart.draw_series(std::iter::once(Rectangle::new(
                [(-0.33, lower[0]), (0.33, upper[0])],
                colour.mix(0.86).filled(),
            )))?;
        } else {
            let polygon = upper
                .iter()
                .enumerate()
                .map(|(index, value)| (index as f64, *value))
                .chain(
                    lower
                        .iter()
                        .enumerate()
                        .rev()
                        .map(|(index, value)| (index as f64, *value)),
                )
                .collect::<Vec<_>>();
            chart.draw_series(std::iter::once(Polygon::new(
                polygon,
                colour.mix(0.86).filled(),
            )))?;
        }
        lower = upper;
    }
    root.present()
        .with_context(|| format!("save {}", path.display()))
}

#[allow(clippy::too_many_arguments)]
fn line_chart(
    rows: &[RevenueRow],
    data: &[[Option<f64>; 6]],
    segments: usize,
    title: &str,
    ylabel: &str,
    reference: Option<f64>,
    note: &str,
    path: &Path,
) -> Result<()> {
    let root = BitMapBackend::new(path, (width(rows.len()), 880)).into_drawing_area();
    root.fill(&WHITE)?;
    let area = header(&root, title, segments, &[], note)?;
    let y_range = range(
        data.iter()
            .flat_map(|values| values[..segments].iter().copied().flatten()),
        reference,
    );
    let quarters = rows
        .iter()
        .map(|row| row.quarter.clone())
        .collect::<Vec<_>>();
    let mut chart = make_chart(&area, rows.len(), y_range, ylabel, &quarters)?;
    if let Some(value) = reference {
        chart.draw_series(std::iter::once(PathElement::new(
            vec![(-0.6, value), (rows.len() as f64 - 0.4, value)],
            GRID.stroke_width(2),
        )))?;
    }
    for (segment, colour) in COLOURS.iter().copied().enumerate().take(segments) {
        // An undefined value breaks the curve rather than joining across it.
        let mut points = Vec::new();
        for (index, values) in data.iter().enumerate() {
            if let Some(value) = values[segment].filter(|value| value.is_finite()) {
                points.push((index as f64, value));
            } else if !points.is_empty() {
                line(
                    &mut chart,
                    points.drain(..),
                    colour,
                    if segment == 5 { 4 } else { 3 },
                )?;
            }
        }
        if !points.is_empty() {
            line(&mut chart, points, colour, if segment == 5 { 4 } else { 3 })?;
        }
    }
    root.present()
        .with_context(|| format!("save {}", path.display()))
}

fn grouped_bars(
    quarters: &[String],
    data: &[[f64; 5]],
    title: &str,
    ylabel: &str,
    contribution: bool,
    path: &Path,
) -> Result<()> {
    let root = BitMapBackend::new(path, (width(quarters.len()), 880)).into_drawing_area();
    root.fill(&WHITE)?;
    let note = if contribution {
        "Contributions use the signed change in total revenue; zero total change gives zero."
    } else {
        "Missing comparisons and zero prior revenue use zero growth."
    };
    let area = header(&root, title, 5, &[], note)?;
    if data.is_empty() {
        let (w, h) = area.dim_in_pixel();
        area.draw(&Text::new(
            "Insufficient data for quarter-over-quarter contribution",
            (w as i32 / 2, h as i32 / 2),
            centred(24, &COLOURS[5]),
        ))?;
    } else {
        let y_range = range(
            data.iter().flatten().copied(),
            contribution.then_some(100.0),
        );
        let mut chart = make_chart(&area, quarters.len(), y_range, ylabel, quarters)?;
        for (index, values) in data.iter().enumerate() {
            for (segment, value) in values.iter().copied().enumerate() {
                let left = index as f64 - 0.375 + segment as f64 * 0.15;
                chart.draw_series(std::iter::once(Rectangle::new(
                    [(left, 0.0), (left + 0.14, value)],
                    COLOURS[segment].filled(),
                )))?;
            }
        }
        chart.draw_series(std::iter::once(PathElement::new(
            vec![(-0.6, 0.0), (quarters.len() as f64 - 0.4, 0.0)],
            COLOURS[5].stroke_width(1),
        )))?;
        if contribution {
            chart.draw_series(std::iter::once(PathElement::new(
                vec![(-0.6, 100.0), (quarters.len() as f64 - 0.4, 100.0)],
                GRID.stroke_width(2),
            )))?;
        }
    }
    root.present()
        .with_context(|| format!("save {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(year: i32, quarter: u8, values: [i64; 6]) -> RevenueRow {
        RevenueRow {
            quarter: format!("Q{quarter} FY{year}"),
            fiscal_year: year,
            quarter_number: quarter,
            data_center: values[0],
            gaming: values[1],
            professional_visualization: values[2],
            automotive: values[3],
            oem_other: values[4],
            total_revenue: values[5],
        }
    }

    #[test]
    fn growth_and_contributions_preserve_signed_changes() {
        let rows = [
            row(2025, 1, [100, 20, 10, 5, 15, 150]),
            row(2025, 2, [120, 15, 10, 10, 15, 170]),
        ];
        let (_, growth) = growth_series(&rows, 1);
        assert_eq!(growth[0], [0.0; 5]);
        assert_eq!(growth[1], [20.0, -25.0, 0.0, 100.0, 0.0]);
        let (_, contribution) = contribution_series(&rows);
        assert_eq!(contribution[0], [100.0, -25.0, 0.0, 25.0, 0.0]);
        assert_eq!(contribution[0].iter().sum::<f64>(), 100.0);
    }

    #[test]
    fn annual_growth_uses_fiscal_quarters_even_with_missing_rows() {
        let rows = [row(2025, 1, [100; 6]), row(2026, 1, [200; 6])];
        let (quarters, growth) = growth_series(&rows, 4);
        assert_eq!(quarters, ["Q1 FY2026"]);
        assert_eq!(growth, [[100.0; 5]]);
        let cagr = cagr_series(&rows);
        assert_eq!(cagr[1], [Some(100.0); 6]);
        assert_eq!(normalized_series(&rows)[1], [Some(200.0); 6]);
        assert_eq!(growth_series(&rows, 1).1, [[0.0; 5]; 2]);
        assert!(contribution_series(&rows).1.is_empty());
    }

    #[test]
    fn zero_baselines_and_zero_total_changes_remain_finite() {
        let rows = [row(2025, 1, [0; 6]), row(2025, 2, [100, -100, 0, 0, 0, 0])];
        assert_eq!(percentages(&rows[0]), [0.0; 5]);
        assert_eq!(normalized_series(&rows), [[None; 6]; 2]);
        assert_eq!(growth_series(&rows, 1).1, [[0.0; 5]; 2]);
        assert_eq!(contribution_series(&rows).1, [[0.0; 5]]);
        assert!(
            cagr_series(&rows)
                .iter()
                .flatten()
                .all(|value| value.is_some_and(f64::is_finite))
        );
    }

    #[test]
    fn cagr_is_annualized_and_negative_current_values_are_undefined() {
        let rows = [
            row(2025, 1, [100; 6]),
            row(2025, 2, [120; 6]),
            row(2025, 3, [-10; 6]),
        ];
        let data = cagr_series(&rows);
        assert!((data[1][0].unwrap() - 107.36).abs() < 1e-9);
        assert_eq!(data[2], [None; 6]);
    }

    #[test]
    fn render_all_charts_and_handle_short_zero_history() -> Result<()> {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("nvidia-chart-test-{}-{unique}", std::process::id()));
        let rows = (0..9)
            .map(|index| {
                row(
                    2024 + index / 4,
                    (index % 4 + 1) as u8,
                    [
                        100 + i64::from(index) * 10,
                        20,
                        10,
                        5,
                        15,
                        150 + i64::from(index) * 10,
                    ],
                )
            })
            .collect::<Vec<_>>();
        let result = (|| -> Result<()> {
            let paths = generate(&rows, &dir)?;
            assert_eq!(paths.len(), 9);
            for path in paths {
                let bytes = fs::read(path)?;
                assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
                assert!(bytes.len() > 1_000);
            }
            let paths = generate(&[row(2025, 1, [0; 6])], &dir)?;
            assert_eq!(paths.len(), 8);
            assert!(!dir.join("growth_rate_yoy.png").exists());
            Ok(())
        })();
        let _ = fs::remove_dir_all(&dir);
        result
    }
}
