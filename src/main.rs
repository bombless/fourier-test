// Cargo.toml 依赖:
// [dependencies]
// wgpu = "0.18"
// pollster = "0.3"
// bytemuck = { version = "1.14", features = ["derive"] }

use std::f64::consts::PI;
use wgpu::util::DeviceExt;

// freq_detect.wgsl (放在同目录下)
const SHADER: &str = r#"
@group(0) @binding(0) var<storage, read> signal: array<f32>;
@group(0) @binding(1) var<storage, read_write> magnitudes: array<f32>;

struct Params {
    sample_rate: f32,
    signal_len: u32,
    freq_start: f32,
    freq_step: f32,
}

@group(0) @binding(2) var<uniform> params: Params;

const PI: f32 = 3.14159265359;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let freq_idx = global_id.x;
    let test_freq = params.freq_start + f32(freq_idx) * params.freq_step;
    
    var sum_x: f32 = 0.0;
    var sum_y: f32 = 0.0;
    
    for (var i: u32 = 0u; i < params.signal_len; i++) {
        let t = f32(i) / params.sample_rate;
        let theta = 2.0 * PI * test_freq * t;
        let amplitude = signal[i];
        
        sum_x += amplitude * cos(theta);
        sum_y += amplitude * sin(theta);
    }
    
    let center_x = sum_x / f32(params.signal_len);
    let center_y = sum_y / f32(params.signal_len);
    
    magnitudes[freq_idx] = sqrt(center_x * center_x + center_y * center_y);
}
"#;

// CPU版本（对比用）
fn detect_frequency_cpu(signal: &[f32], sample_rate: f32) -> (f32, f32) {
    let mut max_magnitude = 0.0f32;
    let mut detected_freq = 0.0f32;
    
    for test_freq in 0..20000 {
        let test_freq = test_freq as f32;
        let mut sum_x = 0.0f32;
        let mut sum_y = 0.0f32;
        
        for (i, &amplitude) in signal.iter().enumerate() {
            let t = i as f32 / sample_rate;
            let theta = 2.0 * PI as f32 * test_freq * t;
            sum_x += amplitude * theta.cos();
            sum_y += amplitude * theta.sin();
        }
        
        let center_x = sum_x / signal.len() as f32;
        let center_y = sum_y / signal.len() as f32;
        let magnitude = (center_x * center_x + center_y * center_y).sqrt();
        
        if magnitude > max_magnitude {
            max_magnitude = magnitude;
            detected_freq = test_freq;
        }
    }
    
    (detected_freq, max_magnitude)
}

// GPU版本
async fn detect_frequency_gpu(signal: &[f32], sample_rate: f32) -> Result<(f32, f32), Box<dyn std::error::Error>> {
    // 初始化GPU
    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .ok_or("Failed to find adapter")?;
    
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default(), None)
        .await?;

    // 创建shader
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Frequency Detection Shader"),
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });

    // 参数
    let num_freqs = 20000u32;
    let params = [sample_rate, signal.len() as _, 0.0f32, 1.0f32];

    // 创建缓冲区
    let signal_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Signal Buffer"),
        contents: bytemuck::cast_slice(signal),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output Buffer"),
        size: (num_freqs as usize * std::mem::size_of::<f32>()) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Params Buffer"),
        contents: bytemuck::cast_slice(&params),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Staging Buffer"),
        size: (num_freqs as usize * std::mem::size_of::<f32>()) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // 创建bind group layout和pipeline
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: signal_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: params_buffer.as_entire_binding(),
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Compute Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some("main"),
        cache: None,
        compilation_options: Default::default(),
    });

    // 创建命令缓冲区并执行
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Command Encoder"),
    });

    {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
        compute_pass.dispatch_workgroups((num_freqs + 255) / 256, 1, 1);
    }

    encoder.copy_buffer_to_buffer(&output_buffer, 0, &staging_buffer, 0, (num_freqs as usize * std::mem::size_of::<f32>()) as u64);

    queue.submit(Some(encoder.finish()));

    // 读取结果
    let buffer_slice = staging_buffer.slice(..);
    buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::Maintain::Wait);

    let data = buffer_slice.get_mapped_range();
    let magnitudes: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
    drop(data);
    staging_buffer.unmap();

    // 找最大值
    let (detected_freq, max_magnitude) = magnitudes
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(idx, &mag)| (idx as f32, mag))
        .unwrap();

    Ok((detected_freq, max_magnitude))
}

fn main() {
    // 生成测试信号: 200Hz
    let sample_rate = 40000.0f32;
    let signal_freq = 200.0f32;
    let signal: Vec<f32> = (0..40000)
        .map(|i| {
            let t = i as f32 / sample_rate;
            (2.0 * PI as f32 * signal_freq * t).sin()
        })
        .collect();

    println!("信号长度: {} 样本", signal.len());
    println!("采样率: {} Hz", sample_rate);
    println!("真实频率: {} Hz\n", signal_freq);

    // CPU版本
    println!("=== CPU版本 ===");
    let start = std::time::Instant::now();
    let (cpu_freq, cpu_mag) = detect_frequency_cpu(&signal, sample_rate);
    let cpu_time = start.elapsed();
    println!("检测频率: {:.1} Hz", cpu_freq);
    println!("最大幅度: {:.6}", cpu_mag);
    println!("耗时: {:?}\n", cpu_time);

    // GPU版本
    println!("=== GPU版本 ===");
    let start = std::time::Instant::now();
    let (gpu_freq, gpu_mag) = pollster::block_on(detect_frequency_gpu(&signal, sample_rate))
        .expect("GPU计算失败");
    let gpu_time = start.elapsed();
    println!("检测频率: {:.1} Hz", gpu_freq);
    println!("最大幅度: {:.6}", gpu_mag);
    println!("耗时: {:?}\n", gpu_time);

    println!("=== 性能对比 ===");
    println!("加速比: {:.1}x", cpu_time.as_secs_f64() / gpu_time.as_secs_f64());
}
