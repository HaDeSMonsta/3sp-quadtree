use color_eyre::Result;
use color_eyre::eyre::{Context, ContextCompat};
use quadtree::tree::DataPoint;
use std::fmt::Debug;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub fn parse<P: AsRef<Path> + Debug>(path: P) -> Result<Vec<DataPoint<f64>>> {
    let mut id_cnt = 0;
    if path.as_ref().is_dir() {
        parse_dir(&path, &mut id_cnt).with_context(|| format!("failed to parse dir: {path:?}"))
    } else {
        parse_file(&path, &mut id_cnt).with_context(|| format!("failed to parse file: {path:?}"))
    }
}

fn parse_dir<P: AsRef<Path>>(path: P, mut id_cnt: &mut usize) -> Result<Vec<DataPoint<f64>>> {
    let mut data = vec![];
    let dir = fs::read_dir(path).context("failed to read dir")?;

    for entry in dir {
        let path = entry.context("invalid file in dir")?.path();

        let mut dir_data = if path.is_dir() {
            parse_dir(path, &mut id_cnt).context("failed to parse recursive dir")?
        } else {
            parse_file(path, &mut id_cnt).context("failed to parse file in dir")?
        };

        data.append(&mut dir_data);
    }

    Ok(data)
}

fn parse_file<P: AsRef<Path> + Debug>(path: P, id_cnt: &mut usize) -> Result<Vec<DataPoint<f64>>> {
    let lines = read(&path)?;

    let idx = lines
        .iter()
        .position(|line| line.contains(&("=".repeat(10))))
        .with_context(|| format!("data has no marker: {path:?}"))?;
    let data_lines = lines
        .into_iter()
        .skip(idx + 3)
        .filter_map(|line| {
            let mut split = line.split(',');
            let x = split.next()?.trim().parse::<f64>().ok()?;
            let y = split.next()?.trim().parse::<f64>().ok()?;

            if !x.is_finite() || !y.is_finite() {
                return None;
            }

            let id = *id_cnt;
            *id_cnt += 1;
            let dp = DataPoint::new_coords(id, x, y);

            Some(dp)
        })
        .collect::<Vec<_>>();

    Ok(data_lines)
}

fn read<P: AsRef<Path>>(path: P) -> Result<Vec<String>> {
    let file = fs::File::open(path).context("failed to open file")?;
    let reader = BufReader::new(file);

    let v = reader
        .lines()
        .map(|line| line.context("failed to read line"))
        .collect::<Result<Vec<_>>>()
        .context("failed to read data")?;

    Ok(v)
}
