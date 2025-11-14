#![feature(stmt_expr_attributes)]

use hound::{WavReader, WavSpec};
use plot_starter::{Chart, Color, Plotter};

use std::f64::consts::PI;

use std::env::args;

fn data(window: f64) -> impl Iterator<Item = (f64, f64)> {
    let mut reader = WavReader::open("./The-Internationale.wav").unwrap();
    let spec = reader.spec();
    let sr = spec.sample_rate;
    let ch = spec.channels as usize;
    println!("sr {sr} ch {ch}");
    let data = reader
        .samples::<i16>()
        .map(|x| x.unwrap() as f64)
        .step_by(ch)
        .take(sr as _) // sample for one second
        .collect::<Vec<_>>();
    #[rustfmt::skip]
    (0..data.len())
        .map(move |i| (i, i as f64 / sr as f64)) // construct (index, time)
        .map(move |(i, t)| (t, data[i] as f64 / 32768.0)) // construct (time, amplitude)
        .map(move |(t, amplitude)| (t / window * 2.0 * PI, amplitude)) // construct (radians, amplitude)
        .map(|(radians, amplitude)| (radians.cos() * amplitude, radians.sin() * amplitude))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let plotter = Plotter::new();

    let window = args().nth(1).unwrap().parse::<f64>().unwrap(); // window is to control the timespan of one circle

    Chart::on(&plotter)
        .data(data(window).into_iter())
        .color(Color::RED);

    plotter.present()
}
