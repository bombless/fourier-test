#![feature(stmt_expr_attributes)]

use hound::{WavReader, WavSpec};
use plot_starter::{Chart, Color, Plotter};

use std::f64::consts::PI;

use std::env::args;

// fn data(window: f64) -> Option<impl Iterator<Item = (f64, f64)>> {
//     let mut reader = WavReader::open("./The-Internationale.wav").unwrap();
//     let spec = reader.spec();
//     let sr = spec.sample_rate;
//     let ch = spec.channels as usize;
//     if window * sr as f64 / 6.0 < 3. {
//         return None;
//     }
//     // println!("sr {sr} ch {ch}");
//     let data = reader
//         .samples::<i16>()
//         .map(|x| x.unwrap() as f64)
//         .step_by(ch)
//         .skip(sr as usize + sr as usize / 6)
//         .take(sr as usize / 6) // sample for one second
//         .collect::<Vec<_>>();
//     #[rustfmt::skip]
//     Some((0..data.len())
//         .map(move |i| (i, i as f64 / sr as f64)) // construct (index, time)
//         .map(move |(i, t)| (t, data[i] as f64 / 32768.0)) // construct (time, amplitude)
//         .map(move |(t, amplitude)| (t * window * 2.0 * PI, amplitude)) // construct (radians, amplitude)
//         .map(|(radians, amplitude)| (radians.cos() * (1.0 + amplitude), radians.sin() * (1.0 + amplitude))))
// }

fn data(frequency: f64) -> Option<impl Iterator<Item = (f64, f64)>> {
    let sr = 40000;
    let duration = 1.0; // 1秒数据
    let n_samples = (sr as f64 * duration) as usize;
    
    // 生成测试信号：200Hz正弦波
    let signal_freq = 200.0;
    
    Some(
        (0..n_samples)
            .map(move |i| {
                let t = i as f64 / sr as f64;
                let amplitude = (2.0 * PI * signal_freq * t).sin();
                (t, amplitude)
            })
            // 关键：直接用amplitude，不要加1.0！
            .map(move |(t, amplitude)| {
                let theta = 2.0 * PI * frequency * t;
                (
                    amplitude * theta.cos(),
                    amplitude * theta.sin(),
                )
            }),
    )
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
    
    let sr = 40000.0;
    let signal_freq = 200.0; // 我们知道信号是200Hz
    
    // 合理的搜索范围：0到奈奎斯特频率
    let mut max_center = 0.0;
    let mut max_freq = 0.0;
    
    // 改用线性步进，精细搜索
    let freq_step = 1.0; // 1Hz步进
    let mut freq = 0.0;
    
    while freq <= sr / 2.0 { // 奈奎斯特频率
        if let Some(data) = data(freq) {
            let data = data.collect::<Vec<_>>();
            let len = center_length(&data);
            
            if freq as i32 % 50 == 0 { // 每50Hz打印一次
                println!("freq: {:.1} Hz, magnitude: {:.6}", freq, len);
            }
            
            if len > max_center {
                max_center = len;
                max_freq = freq;
            }
        }
        freq += freq_step;
    }
    
    println!("\n检测到的频率: {:.1} Hz (实际: {:.1} Hz)", max_freq, signal_freq);
    println!("最大幅度: {:.6}", max_center);
    
    // 可视化最佳频率
    Chart::on(&plotter)
        .data(data(max_freq).unwrap())
        .color(Color::RED);
    
    plotter.present()
}
