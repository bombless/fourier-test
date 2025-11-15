#![feature(stmt_expr_attributes)]

use hound::{WavReader, WavSpec};
use plot_starter::{Chart, Color, Plotter};

use std::f64::consts::PI;

use std::env::args;

fn data(window: f64) -> Option<impl Iterator<Item = (f64, f64)>> {
    let mut reader = WavReader::open("./The-Internationale.wav").unwrap();
    let spec = reader.spec();
    let sr = spec.sample_rate;
    let ch = spec.channels as usize;
    if window * sr as f64 / 6.0 < 1. {
        return None;
    }
    // println!("sr {sr} ch {ch}");
    let data = reader
        .samples::<i16>()
        .map(|x| x.unwrap() as f64)
        .step_by(ch)
        .skip(sr as usize + sr as usize / 6)
        .take(sr as usize / 6) // sample for one second
        .collect::<Vec<_>>();
    #[rustfmt::skip]
    Some((0..data.len())
        .map(move |i| (i, i as f64 / sr as f64)) // construct (index, time)
        .map(move |(i, t)| (t, data[i] as f64 / 32768.0)) // construct (time, amplitude)
        .map(move |(t, amplitude)| (t * window * 2.0 * PI, amplitude)) // construct (radians, amplitude)
        .map(|(radians, amplitude)| (radians.cos() * (1.0 + amplitude), radians.sin() * (1.0 + amplitude))))
}

fn center_length(data: &[(f64, f64)]) -> f64 {
    let factor = 1.0 / data.len() as f64;
    let center = data
        .iter()
        .cloned()
        .reduce(|(sx, sy), (x, y)| (sx + x * factor, sy + y * factor))
        .unwrap();
    (center.0 * center.0 + center.1 * center.1).sqrt()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let plotter = Plotter::new();

    // let window = args().nth(1).unwrap().parse::<f64>().unwrap(); // window is to control the timespan of one circle

    let mut i = 0.00000001;
    let mut max_center = 0.0;
    let mut max_window = 0.0;
    let mut second_max_window = 0.0;
    while i < 1000000.0 {
        let data = if let Some(data) = data(i) {
            data
        } else {
            i *= 10.0;
            continue;
        };
        let data = &*data.collect::<Vec<_>>();
        let len = center_length(data);
        println!("({i}, {len})");
        if len > max_center {
            max_center = len;
            second_max_window = max_window;
            max_window = i;
        }
        i *= 10.0;
    }
    println!("max_window {max_window} second_max_window {second_max_window}");
    Chart::on(&plotter)
        .data(data(max_window).unwrap())
        .color(Color::RED);

    plotter.present()
}
