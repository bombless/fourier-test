#![feature(stmt_expr_attributes)]

use hound::{WavReader, WavSpec};
use plot_starter::{Chart, Color, Plotter};

use std::f64::consts::PI;

use std::env::args;

// 生成测试信号（不含任何频率检测逻辑）
fn generate_signal() -> (usize, Vec<f64>) {
    let sr = 40000;
    let signal_freq = 440; // 测试信号频率
    
    (sr, (0..sr)
        .map(|i| {
            let t = i as f64 / sr as f64;
            (2.0 * PI * signal_freq as f64 * t).sin()
        })
        .collect())
}

// 从WAV文件读取信号
fn load_signal_from_wav() -> (usize, Vec<f64>) {
    let mut reader = WavReader::open("./The-Internationale.wav").unwrap();
    let spec = reader.spec();
    let sr = spec.sample_rate as usize;
    let ch = spec.channels as usize;
    
    (sr, reader
        .samples::<i16>()
        .map(|x| x.unwrap() as f64 / 32768.0)
        .step_by(ch)
        .skip(sr + sr / 6)
        .take(sr / 6)
        .collect())
}

// 将信号转换为极坐标点（用于可视化）
fn signal_to_polar(signal: &[f64], test_freq: f64, sample_rate: f64) -> Vec<(f64, f64)> {
    signal
        .iter()
        .enumerate()
        .map(|(i, &amplitude)| {
            let t = i as f64 / sample_rate;
            let theta = 2.0 * PI * test_freq * t;
            (
                amplitude * theta.cos(),
                amplitude * theta.sin(),
            )
        })
        .collect()
}

// 计算重心到原点的距离（频率检测核心算法）
fn calculate_centroid_magnitude(signal: &[f64], test_freq: f64, sample_rate: f64) -> f64 {
    let n = signal.len() as f64;
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    
    for (i, &amplitude) in signal.iter().enumerate() {
        let t = i as f64 / sample_rate;
        let theta = 2.0 * PI * test_freq * t;
        sum_x += amplitude * theta.cos();
        sum_y += amplitude * theta.sin();
    }
    
    let center_x = sum_x / n;
    let center_y = sum_y / n;
    
    (center_x * center_x + center_y * center_y).sqrt()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let plotter = Plotter::new();
    
    // 1. 生成或加载信号
    let (sample_rate, signal) = load_signal_from_wav();
    // let (sample_rate, signal) = generate_signal();
    let sample_rate = sample_rate as f64;

    
    // 2. 搜索频率
    let mut max_magnitude = 0.0;
    let mut detected_freq = 0.0;
    
    let freq_step = 1.0; // 1Hz步进
    let mut test_freq = 0.0;
    
    println!("开始频率扫描...\n");
    
    while test_freq <= sample_rate / 2.0 {
        let magnitude = calculate_centroid_magnitude(&signal, test_freq, sample_rate);
        
        if test_freq as i32 % 50 == 0 {
            println!("测试频率: {:6.1} Hz, 幅度: {:.6}", test_freq, magnitude);
        }
        
        if magnitude > max_magnitude {
            max_magnitude = magnitude;
            detected_freq = test_freq;
        }
        
        test_freq += freq_step;
    }
    
    println!("\n===================");
    println!("检测到的频率: {:.1} Hz", detected_freq);
    println!("最大幅度: {:.6}", max_magnitude);
    println!("===================\n");
    
    // 3. 可视化检测到的频率对应的极坐标图
    let polar_points = signal_to_polar(&signal, detected_freq, sample_rate);
    Chart::on(&plotter)
        .data(polar_points.into_iter())
        .color(Color::RED);
    
    plotter.present()
}
