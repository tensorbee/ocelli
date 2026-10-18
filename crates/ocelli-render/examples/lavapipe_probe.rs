//! F-X002 layer 2. Ask the pinned wgpu for a fallback adapter and name it.
//!
//! Exit 0 when the name, driver or driver info is lavapipe or llvmpipe.
//! Exit 1 when an adapter comes back and it is not. Exit 2 when none comes
//! back. This is not a GPU-less browser session and it is not the oracle.
//! `gate gpu` does not run this example.

fn main() {
    let code = pollster::block_on(probe());
    std::process::exit(code);
}

async fn probe() -> i32 {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let options = wgpu::RequestAdapterOptions {
        force_fallback_adapter: true,
        ..Default::default()
    };
    let adapter = match instance.request_adapter(&options).await {
        Ok(adapter) => adapter,
        Err(error) => {
            eprintln!("no fallback adapter: {error}");
            return 2;
        }
    };
    let info = adapter.get_info();
    println!("{}", report(&info));
    if ocelli_render::caps::fallback_is_lavapipe(&info.name, &info.driver, &info.driver_info) {
        0
    } else {
        1
    }
}

fn report(info: &wgpu::AdapterInfo) -> String {
    format!(
        "{{\"name\":\"{}\",\"driver\":\"{}\",\"driver_info\":\"{}\",\"device_type\":\"{}\",\"backend\":\"{}\"}}",
        json_escape(&info.name),
        json_escape(&info.driver),
        json_escape(&info.driver_info),
        device_type_name(info.device_type),
        info.backend.to_str(),
    )
}

fn device_type_name(device_type: wgpu::DeviceType) -> &'static str {
    match device_type {
        wgpu::DeviceType::Other => "other",
        wgpu::DeviceType::IntegratedGpu => "integrated",
        wgpu::DeviceType::DiscreteGpu => "discrete",
        wgpu::DeviceType::VirtualGpu => "virtual",
        wgpu::DeviceType::Cpu => "cpu",
    }
}

fn json_escape(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            other if other.is_control() => out.push(' '),
            other => out.push(other),
        }
    }
    out
}
