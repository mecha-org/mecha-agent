use anyhow::Result;
use opentelemetry::{
    global,
    metrics::{Meter, Unit},
    Key,
};
use sysinfo::{CpuRefreshKind, RefreshKind, System};

use crate::config::init_otlp_configuration;

async fn initialize_metrics() -> Result<bool> {
    let fn_name = "initialize_custom_metrics";
    let _meter_provider = match init_otlp_configuration() {
        Ok(provider) => provider,
        Err(e) => {
            println!(
                "{}: error initializing otlp configuration: {:?}",
                fn_name, e
            );
            return Ok(false);
        }
    };
    let meter = global::meter("ex.com/basic");
    match collect_memory_utilization(meter) {
        Ok(_) => println!("memory utilization collected"),
        Err(e) => println!("error collecting memory utilization: {:?}", e),
    }
    Ok(true)
}
/*
SYSTEM_CPU_UTILIZATION("system_cpu_time_seconds_total"),
SYSTEM_MEMORY_USAGE("system_memory_usage_bytes"),
SYSTEM_CPU_LOAD_AVERAGE_15M("system_cpu_load_average_15m_ratio"),
SYSTEM_NETWORK_IO("system_network_io_bytes_total"),
SYSTEM_DISK_IO("system_disk_io_bytes_total"),
SYSTEM_FILESYSTEMS_USAGE("system_filesystem_usage_bytes");
*/

// SYSTEM_CPU_UTILIZATION("system_cpu_time_seconds_total"),
fn collect_cpu_utilization(meter: Meter) -> Result<()> {
    let cpu_utilization_obs_counter = meter
        .f64_observable_gauge("system.cpu.utilization")
        .with_description("Difference in system.cpu.time since the last measurement, divided by the elapsed time and number of logical CPUs.")
        .with_unit(Unit::new("1"))
        .init();
    match meter.register_callback(&[cpu_utilization_obs_counter.as_any()], move |observer| {
        let s =
            System::new_with_specifics(RefreshKind::new().with_cpu(CpuRefreshKind::everything()));
        let mut total_cpu_usage: f32 = 0.0;
        let cpus = s.cpus();
        for cpu in cpus {
            println!("{}%", cpu.cpu_usage());
            total_cpu_usage += cpu.cpu_usage();
        }
        let attrs = vec![
            Key::new("system.cpu.state").string("system"), //todo: value confirm with shoaib
            Key::new("system.cpu.logical_number").i64(8),  //todo: value confirm with shoaib
        ];
        observer.observe_f64(&cpu_utilization_obs_counter, total_cpu_usage as f64, &attrs);
    }) {
        Ok(_) => println!("callback registered"),
        Err(e) => println!("error registering callback: {:?}", e),
    };
    Ok(())
}

// SYSTEM_MEMORY_USAGE("system_memory_usage_bytes"),
fn collect_memory_usage(meter: Meter) -> Result<()> {
    let memory_utilization_obs_counter = meter
        .f64_observable_up_down_counter("mecha.memory.usage")
        .with_description("Reports memory in use by state.")
        .with_unit(Unit::new("By"))
        .init();
    let cloned_memory_utilization_obs_counter = memory_utilization_obs_counter.clone();
    match meter.register_callback(
        &[cloned_memory_utilization_obs_counter.as_any()],
        move |observer| {
            let used_mem = System::new_all().used_memory();
            let attrs = vec![Key::new("system.memory.state").string("used")];
            observer.observe_f64(&memory_utilization_obs_counter, used_mem as f64, &attrs);
        },
    ) {
        Ok(_) => println!("callback registered"),
        Err(e) => println!("error registering callback: {:?}", e),
    };
    Ok(())
}

//SYSTEM_CPU_LOAD_AVERAGE_15M("system_cpu_load_average_15m_ratio"),
fn collect_cpu_load_average(meter: Meter) -> Result<()> {
    let cpu_utilization_obs_counter = meter
        .f64_observable_gauge("system.linux.cpu.load_15m")
        .with_description("Difference in system.cpu.time since the last measurement, divided by the elapsed time and number of logical CPUs.")
        .with_unit(Unit::new("1"))
        .init();
    match meter.register_callback(&[cpu_utilization_obs_counter.as_any()], move |observer| {
        let s =
            System::new_with_specifics(RefreshKind::new().with_cpu(CpuRefreshKind::everything()));
        let mut total_cpu_usage: f32 = 0.0;
        let cpus = s.cpus();
        for cpu in cpus {
            println!("{}%", cpu.cpu_usage());
            total_cpu_usage += cpu.cpu_usage();
        }
        let attrs = vec![
            Key::new("system.cpu.state").string("system"), //todo: value confirm with shoaib
            Key::new("system.cpu.logical_number").i64(8),  //todo: value confirm with shoaib
        ];
        observer.observe_f64(&cpu_utilization_obs_counter, total_cpu_usage as f64, &attrs);
    }) {
        Ok(_) => println!("callback registered"),
        Err(e) => println!("error registering callback: {:?}", e),
    };
    Ok(())
}

// SYSTEM_NETWORK_IO("system_network_io_bytes_total"),
fn collect_network_io(meter: Meter) -> Result<()> {
    let memory_utilization_obs_counter = meter
        .f64_observable_up_down_counter("mecha.memory.usage")
        .with_description("Reports memory in use by state.")
        .with_unit(Unit::new("By"))
        .init();
    let cloned_memory_utilization_obs_counter = memory_utilization_obs_counter.clone();
    match meter.register_callback(
        &[cloned_memory_utilization_obs_counter.as_any()],
        move |observer| {
            let used_mem = System::new_all().used_memory();
            let attrs = vec![Key::new("system.memory.state").string("used")];
            observer.observe_f64(&memory_utilization_obs_counter, used_mem as f64, &attrs);
        },
    ) {
        Ok(_) => println!("callback registered"),
        Err(e) => println!("error registering callback: {:?}", e),
    };
    Ok(())
}
