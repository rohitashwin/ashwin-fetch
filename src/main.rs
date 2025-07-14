use chrono::Duration;
use std::collections::HashMap;
use std::fmt::Debug;
use std::process::ExitCode;
use sysinfo::Motherboard;
use sysinfo::System;
use wgpu::Backends;
use wgpu::Instance;
use wgpu::InstanceDescriptor;

const LOGO_HEIGHT: usize = 9;
const LOGO_WIDTH: usize = 32;
const LOGO: [&str; LOGO_HEIGHT] = [
    "       :#.                      ",
    "       :#-:****************+    ",
    "         -::::::::.......:::    ",
    "   .#*               -**=:.     ",
    "    #-::            =%%=:.      ",
    "     --::.        :*%#::        ",
    "       -:::.    .=%%-:          ",
    "         :::=######:.           ",
    "          .::::::..             ",
];

struct CpuInfo {
    num_cores: usize,
    avg_usage: f64,
    max_frequency_mhz: f64,
}

impl Debug for CpuInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CpuInfo")
            .field("num_cores", &self.num_cores)
            .field("avg_usage", &self.avg_usage)
            .field("max_frequency_mhz", &self.max_frequency_mhz)
            .finish()
    }
}

struct GpuInfo {
    device_index: usize,
    gpu_name: String,
}

impl Debug for GpuInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GpuInfo")
            .field("device_index", &self.device_index)
            .field("gpu_name", &self.gpu_name)
            .finish()
    }
}

struct OutputInfo<'a> {
    username: String,
    hostname: String,
    os: String,
    serial_number: String,
    kernel: String,
    uptime: usize,
    cpu: HashMap<&'a str, CpuInfo>,
    gpu: Vec<GpuInfo>,
    memory_used_mb: usize,
    memory_total_mb: usize,
}

fn get_username() -> String {
    return whoami::username();
}

fn get_hostname() -> String {
    return whoami::fallible::hostname().unwrap_or(String::from("unknown"));
}

fn get_os_name() -> String {
    return whoami::distro();
}

fn get_serial_number() -> String {
    return Motherboard::new()
        .and_then(|x| x.serial_number())
        .unwrap_or("xxxxxxxxxx".to_string());
}

fn kernel() -> String {
    return System::kernel_long_version();
}

fn get_uptime() -> usize {
    return System::uptime() as usize;
}

fn get_cpu_info<'a>(sys: &'a System) -> HashMap<&'a str, CpuInfo> {
    let mut cpu_info_map = HashMap::<&'a str, CpuInfo>::new();
    for cpu in sys.cpus() {
        let entry = cpu_info_map.entry(cpu.brand()).or_insert(CpuInfo {
            num_cores: 0,
            avg_usage: 0.0 as f64,
            max_frequency_mhz: 0.0 as f64,
        });
        entry.num_cores += 1;
        entry.avg_usage += cpu.cpu_usage() as f64;
        if cpu.frequency() as f64 > entry.max_frequency_mhz {
            entry.max_frequency_mhz = cpu.frequency() as f64;
        }
    }
    for (_, val) in &mut cpu_info_map {
        val.avg_usage /= val.num_cores as f64;
    }
    return cpu_info_map;
}

fn get_gpu_info() -> Vec<GpuInfo> {
    let mut instance_descriptor = InstanceDescriptor::default();
    instance_descriptor.backends = Backends::all();
    let instance = Instance::new(&instance_descriptor);
    let adapters = instance.enumerate_adapters(Backends::all());
    let mut gpu_infos = vec![];
    for (idx, adapter) in adapters.iter().enumerate() {
        let info = adapter.get_info();
        if info.device_type == wgpu::DeviceType::Other || info.device_type == wgpu::DeviceType::Cpu {
            continue;
        }
        gpu_infos.push(GpuInfo {
            device_index: idx,
            gpu_name: match info.device_type {
                wgpu::DeviceType::IntegratedGpu => format!("{} (Integrated GPU)", info.name),
                wgpu::DeviceType::DiscreteGpu => format!("{} (Discrete GPU)", info.name),
                wgpu::DeviceType::VirtualGpu => format!("{} (Virtual GPU)", info.name),
                wgpu::DeviceType::Cpu => format!("{} (Software Rasterizer)", info.name),
                wgpu::DeviceType::Other => format!("{} (unknown gpu type)", info.name),
            },
        });
    }
    gpu_infos.sort_by(|x, y| x.device_index.cmp(&y.device_index));
    return gpu_infos;
}

fn get_used_memory(sys: &System) -> usize {
    return sys.used_memory() as usize;
}

fn get_total_memory(sys: &System) -> usize {
    return sys.total_memory() as usize;
}

fn convert_unix_to_human_string(unix_time: usize) -> String {
    let duration = Duration::seconds(unix_time as i64);
    let days = duration.num_days();
    let hours = duration.num_hours() % 24;
    let minutes = duration.num_minutes() % 60;

    if days > 0 {
        return format!("{}d {}h {}m", days, hours, minutes);
    } else if hours > 0 {
        return format!("{}h {}m", hours, minutes);
    } else {
        return format!("{}m", minutes);
    }
}

fn print_all_info(output_info: &OutputInfo) {
    let mut output_info_vec = vec![
        format!("{}@{}", output_info.username, output_info.hostname),
        format!("{}", "-".repeat(output_info.username.len() + output_info.hostname.len() + 1)),
        format!("OS:        {}", output_info.os),
        format!("Serial:    {}", output_info.serial_number),
        format!("Kernel:    {}", output_info.kernel),
        format!("Uptime:    {}", convert_unix_to_human_string(output_info.uptime)),
    ];
    for (cpu_brand, cpu_info) in &output_info.cpu {
        output_info_vec.push(format!(
            "CPU:       {} - {} cores, {:.2}% avg, {:.2} MHz (max)",
            cpu_brand, cpu_info.num_cores, cpu_info.avg_usage, cpu_info.max_frequency_mhz
        ));
    }
    for gpu_info in &output_info.gpu {
        output_info_vec.push(format!(
            "GPU {:.>3}:   {}",
            gpu_info.device_index, gpu_info.gpu_name
        ));
    }
    output_info_vec.push(format!(
        "Memory:    {}/{} MB used",
        output_info.memory_used_mb, output_info.memory_total_mb
    ));
    println!();
    for (idx, line) in output_info_vec.iter().enumerate() {
        if idx < LOGO_HEIGHT {
            println!("{}{}", LOGO[idx], line);
        } else {
            println!("{}{}", " ".repeat(LOGO_WIDTH), line);
        }
    }
    if output_info_vec.len() < LOGO_HEIGHT {
        for i in output_info_vec.len()..LOGO_HEIGHT {
            println!("{}", LOGO[i]);
        }
    }
    println!();
}

fn main() -> ExitCode {
    if !sysinfo::IS_SUPPORTED_SYSTEM {
        println!("System not supported. Aborting.");
        return ExitCode::from(1);
    }

    let mut sys = System::new_all();
    std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    sys.refresh_cpu_all();

    let output_info = OutputInfo {
        username: get_username(),
        hostname: get_hostname(),
        os: get_os_name(),
        serial_number: get_serial_number(),
        kernel: kernel(),
        uptime: get_uptime(),
        cpu: get_cpu_info(&sys),
        gpu: get_gpu_info(),
        memory_used_mb: get_used_memory(&sys) / 1024 / 1024,
        memory_total_mb: get_total_memory(&sys) / 1024 / 1024,
    };

    print_all_info(&output_info);

    return ExitCode::from(0);
}
