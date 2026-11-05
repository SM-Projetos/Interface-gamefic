//! Detecção de kernel, arquitetura, processador, GPUs e suporte Vulkan —
//! porto dos módulos `cpu.rs`, `gpu.rs` e `vulkan.rs` do repositório
//! Gamefic, Rust puro (sem Tauri/serde).
//!
//! Detecção degradada é honesta: o que não puder ser consultado volta
//! como None/vetor vazio e a interface exibe "Não disponível" — nada é
//! inventado como medição real.
//!
//! Consumo: `main.rs` alimenta o global Slint `KernelData` (aba Kernels
//! e Drivers) com os valores formatados.

use super::spawn::comando;
use super::tools;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

pub fn kernel_running() -> Option<String> {
    super::os::detect_kernel()
}

pub fn detect_arch() -> Option<String> {
    super::os::detect_arch()
}

pub fn detect_distro() -> Option<super::DistroInfo> {
    super::os::detect_distro()
}

/// dados do processador suficientes para a aba Kernels e Drivers
pub struct CpuInfo {
    pub model: Option<String>,
    pub cores_physical: Option<u32>,
    pub threads_logical: Option<u32>,
    pub base_freq_mhz: Option<u32>,
}

pub struct GpuInfo {
    pub pci_address: Option<String>,
    pub vendor_id: Option<String>,
    pub device_id: Option<String>,
    pub vendor_name: Option<String>,
    pub device_name: Option<String>,
    pub driver: Option<String>,
}

pub struct VulkanGpu {
    pub device_name: Option<String>,
    pub driver_name: Option<String>,
}

pub struct VulkanInfo {
    pub gpus: Vec<VulkanGpu>,
}

pub struct KernelDriversSnapshot {
    pub distro: Option<super::DistroInfo>,
    pub kernel: Option<String>,
    pub arch: Option<String>,
    pub cpu: Option<CpuInfo>,
    pub gpus: Vec<GpuInfo>,
    pub vulkan: Option<VulkanInfo>,
    pub ferramentas: Vec<super::ToolInfo>,
}

/// detecção completa da aba; consultas vivas, sequência igual ao app
/// nativo do repositório (caro o suficiente para thread de fundo)
pub fn snapshot() -> KernelDriversSnapshot {
    KernelDriversSnapshot {
        distro: detect_distro(),
        kernel: kernel_running(),
        arch: detect_arch(),
        cpu: detect_cpu(),
        gpus: detect_gpus(),
        vulkan: detect_vulkan(),
        ferramentas: tools::detect(),
    }
}

// ============================================================================
// Processador (/proc/cpuinfo + /sys/devices/system/cpu)
// ============================================================================

pub fn detect_cpu() -> Option<CpuInfo> {
    let info = parse_cpuinfo();

    if info.model.is_none() && info.threads_logical.map(|t| t == 0).unwrap_or(true) {
        return None;
    }

    Some(info)
}

fn parse_cpuinfo() -> CpuInfo {
    let text = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();

    let mut model: Option<String> = None;
    let mut processor_count: u32 = 0;

    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());

        match key {
            "model name" if model.is_none() => model = Some(value.to_string()),
            "processor" if value.chars().all(|c| c.is_ascii_digit()) => processor_count += 1,
            _ => {}
        }
    }

    let threads_logical = if processor_count > 0 {
        Some(processor_count)
    } else {
        None
    };

    CpuInfo {
        model,
        cores_physical: physical_core_count(),
        threads_logical,
        base_freq_mhz: base_freq(),
    }
}

fn cpu_dir() -> Option<&'static Path> {
    let path = Path::new("/sys/devices/system/cpu");
    if path.is_dir() { Some(path) } else { None }
}

fn physical_core_count() -> Option<u32> {
    let base = cpu_dir()?;

    let mut pairs: HashSet<(String, String)> = HashSet::new();

    if let Ok(entries) = fs::read_dir(base) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !looks_like_cpu(&name) {
                continue;
            }

            let topo = entry.path().join("topology");
            let core = topo.join("core_id");
            let package = topo.join("physical_package_id");

            if let (Ok(core), Ok(package)) = (fs::read_to_string(core), fs::read_to_string(package))
            {
                pairs.insert((package.trim().to_string(), core.trim().to_string()));
            }
        }
    }

    if !pairs.is_empty() {
        return Some(pairs.len() as u32);
    }

    None
}

fn looks_like_cpu(name: &str) -> bool {
    if !name.starts_with("cpu") {
        return false;
    }
    name[3..].chars().all(|c| c.is_ascii_digit())
}

fn base_freq() -> Option<u32> {
    let text = fs::read_to_string("/proc/cpuinfo").ok()?;
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        if key.trim() == "cpu MHz" || key.trim() == "clock" {
            let mhz: u32 = value.trim().parse().unwrap_or(0);
            if mhz > 0 {
                return Some(mhz);
            }
        }
    }

    let max_scaling = max_scaling_freq().map(|v| (v / 1000) as u32);
    if let Some(max) = max_scaling {
        return Some(max / 2);
    }

    None
}

fn max_scaling_freq() -> Option<u64> {
    let base = cpu_dir()?;
    let mut max_khz: u64 = 0;

    if let Ok(entries) = fs::read_dir(base) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !looks_like_cpu(&name) {
                continue;
            }
            if let Ok(raw) = fs::read_to_string(entry.path().join("cpufreq/cpuinfo_max_freq")) {
                let khz: u64 = raw.trim().parse().unwrap_or(0);
                if khz > max_khz {
                    max_khz = khz;
                }
            }
        }
    }

    if max_khz > 0 { Some(max_khz) } else { None }
}

// ============================================================================
// GPUs (sysfs PCI + pci.ids + lspci -k -mm)
// ============================================================================

pub fn detect_gpus() -> Vec<GpuInfo> {
    let mut gpus: Vec<GpuInfo> = Vec::new();

    if let Ok(entries) = fs::read_dir("/sys/bus/pci/devices") {
        for entry in entries.flatten() {
            let path = entry.path();

            let class = fs::read_to_string(path.join("class"))
                .map(|c| c.trim().to_string())
                .unwrap_or_default();

            if !is_vga_or_display(&class) {
                continue;
            }

            gpus.push(inspect_device(&path));
        }
    }

    let pci_ids = load_pci_ids();
    if let Some((vendors, devices)) = &pci_ids {
        for gpu in &mut gpus {
            if let Some(vendor) = &gpu.vendor_id {
                if let Some(vendor_name) = vendors.get(vendor) {
                    gpu.vendor_name = Some(vendor_name.clone());
                }
                if let Some(device_id) = &gpu.device_id {
                    if let Some(devices_of_vendor) = devices.get(vendor) {
                        if let Some(device_name) = devices_of_vendor.get(device_id) {
                            gpu.device_name = Some(device_name.clone());
                        }
                    }
                }
            }
        }
    }

    if let Ok(output) = run("lspci", &["-k", "-mm"]) {
        apply_lspci_output(gpus.as_mut_slice(), &output);
    }

    gpus.sort_by(|a, b| a.pci_address.cmp(&b.pci_address));
    gpus
}

fn is_vga_or_display(class: &str) -> bool {
    class.starts_with("0x0300") || class.starts_with("0x0302")
}

fn inspect_device(path: &Path) -> GpuInfo {
    GpuInfo {
        pci_address: Some(
            path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
        ),
        vendor_id: hex_value(fs::read_to_string(path.join("vendor"))),
        device_id: hex_value(fs::read_to_string(path.join("device"))),
        vendor_name: None,
        device_name: None,
        driver: driver_of(path),
    }
}

fn hex_value(raw: std::io::Result<String>) -> Option<String> {
    let text = raw.ok()?;
    let trimmed = text.trim().trim_start_matches("0x").to_uppercase();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn driver_of(path: &Path) -> Option<String> {
    fs::read_link(path.join("driver"))
        .ok()?
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
}

type PciIds = (
    HashMap<String, String>,
    HashMap<String, HashMap<String, String>>,
);

fn load_pci_ids() -> Option<PciIds> {
    let text = fs::read_to_string("/usr/share/hwdata/pci.ids")
        .or_else(|_| fs::read_to_string("/usr/share/misc/pci.ids"))
        .ok()?;

    let mut vendors: HashMap<String, String> = HashMap::new();
    let mut devices: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut current_vendor: Option<String> = None;

    for line in text.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if !line.starts_with(|c: char| c == '\t' || c == ' ') {
            let Some((id, name)) = line.split_once(char::is_whitespace) else {
                continue;
            };
            if id.len() != 4 || !id.chars().all(|c| c.is_ascii_hexdigit()) {
                continue;
            }
            vendors.insert(id.to_uppercase(), name.trim().to_string());
            current_vendor = Some(id.to_uppercase());
            continue;
        }

        if line.starts_with("\t\t") || line.starts_with("  ") {
            continue;
        }

        let (Some(vendor), Some((id, name))) = (
            current_vendor.as_deref(),
            line.trim_start().split_once(char::is_whitespace),
        ) else {
            continue;
        };

        if id.len() != 4 || !id.chars().all(|c| c.is_ascii_hexdigit()) {
            continue;
        }

        devices
            .entry(vendor.to_string())
            .or_default()
            .insert(id.to_uppercase(), name.trim().to_string());
    }

    Some((vendors, devices))
}

fn run(program: &str, args: &[&str]) -> Result<String, std::io::Error> {
    let output = comando(program).args(args).output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "programa externo retornou status de erro",
        ))
    }
}

fn apply_lspci_output(gpus: &mut [GpuInfo], output: &str) {
    let mut current_addr: Option<String> = None;

    for line in output.lines() {
        if let Some(addr_end) = line.find(" VGA compatible controller:") {
            current_addr = Some(line[..addr_end].trim().to_string());
        } else if let Some(addr_end) = line.find(" Display controller:") {
            current_addr = Some(line[..addr_end].trim().to_string());
        }

        if let (Some(addr), driver) = (current_addr.clone(), extract_driver(line)) {
            if let Some(gpu) = gpus
                .iter_mut()
                .find(|g| g.pci_address.as_deref() == Some(addr.as_str()))
            {
                if gpu.driver.is_none() {
                    gpu.driver = driver;
                }
            }
        }
    }
}

fn extract_driver(line: &str) -> Option<String> {
    let driver = line.trim_start().strip_prefix("Kernel driver in use: ")?;
    Some(driver.trim().to_string())
}

// ============================================================================
// Vulkan (resumo textual do vulkaninfo)
// ============================================================================

pub fn detect_vulkan() -> Option<VulkanInfo> {
    let output = comando("vulkaninfo").arg("--summary").output().ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout).into_owned();

    let mut gpus: Vec<VulkanGpu> = Vec::new();
    let mut current_device: Option<String> = None;
    let mut current_driver: Option<String> = None;

    for line in text.lines() {
        let trimmed = line.trim();

        if let Some(rest) = trimmed.strip_prefix("deviceName") {
            if let Some(previous) = current_device.take() {
                gpus.push(VulkanGpu {
                    device_name: Some(previous),
                    driver_name: current_driver.take(),
                });
            }
            current_device = value_of(rest);
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("driverName") {
            current_driver = value_of(rest);
            continue;
        }

        if trimmed.starts_with("deviceUUID")
            || trimmed.starts_with("apiVersion")
            || trimmed.starts_with("deviceType")
        {
            if let Some(device) = current_device.take() {
                gpus.push(VulkanGpu {
                    device_name: Some(device),
                    driver_name: current_driver.take(),
                });
            }
        }
    }

    if let Some(device) = current_device.take() {
        gpus.push(VulkanGpu {
            device_name: Some(device),
            driver_name: current_driver.take(),
        });
    }

    if gpus.is_empty() {
        None
    } else {
        Some(VulkanInfo { gpus })
    }
}

fn value_of(rest: &str) -> Option<String> {
    let (_, value) = rest.split_once('=')?;
    let value = value.trim();
    if value.is_empty() {
        None
    } else if let Some((first, _)) = value.split_once('|') {
        Some(first.trim().to_string())
    } else {
        Some(value.to_string())
    }
}
