use fst::Map;
use fst::Streamer;

use plotters::prelude::*;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let project_root = env::var("PROJECT_ROOT").unwrap();
    let evaluations_fst_path = format!("{}/fst/evaluations.fst", project_root);

    let binding = Map::new(std::fs::read(evaluations_fst_path)?)?;
    let mut evaluations = binding.stream();

    let mut data: Vec<(u64, u64)> = Vec::new();
    while let Some((key, value)) = &evaluations.next() {
        let key_str = std::str::from_utf8(key)?;
        // let key_num: u64 = key_str.parse()?;
        let key_len = key_str.len().try_into().unwrap();
        data.push((*value, key_len));
    }

    data.sort_by_key(|&(key, _)| key);

    let graph_path = format!("{}/evaluations.png", project_root);
    let root = BitMapBackend::new(&graph_path, (4800, 3200)).into_drawing_area();
    root.fill(&RGBColor(30, 30, 30))?;

    let mut chart = ChartBuilder::on(&root)
        .caption("Evaluations", ("sans-serif", 50).into_font().color(&WHITE))
        .margin(10)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(
            0u64..data.last().unwrap().0,
            0u64..data.iter().map(|&(_, v)| v).max().unwrap(),
        )?;

    chart
        .configure_mesh()
        .x_labels(10)
        .y_labels(10)
        .axis_style(&WHITE)
        .label_style(("sans-serif", 15).into_font().color(&WHITE))
        .light_line_style(&WHITE.mix(0.2))
        .bold_line_style(&WHITE.mix(0.5))
        .draw()?;

    chart.draw_series(data.iter().map(|&(x, y)| {
        Circle::new((x, y), 1, ShapeStyle::from(&RGBColor(220, 70, 70)).filled())
    }))?;

    return Ok(());
}
