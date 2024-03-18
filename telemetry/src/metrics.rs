use anyhow::Result;
use opentelemetry::{
    global,
    metrics::{Meter, Unit},
    Key,
};
use sysinfo::{CpuRefreshKind, Disks, Networks, RefreshKind, System};

use crate::config::init_otlp_configuration;

pub async fn initialize_metrics() -> Result<bool> {
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
    match collect_memory_usage(meter.clone()) {
        Ok(_) => println!("memory utilization collected"),
        Err(e) => println!("error collecting memory utilization: {:?}", e),
    }
    match collect_cpu_utilization(meter.clone()) {
        Ok(_) => println!("cpu utilization collected"),
        Err(e) => println!("error collecting cpu utilization: {:?}", e),
    }
    match collect_cpu_load_average(meter.clone()) {
        Ok(_) => println!("cpu load average collected"),
        Err(e) => println!("error collecting cpu load average: {:?}", e),
    }
    match collect_network_io(meter.clone()) {
        Ok(_) => println!("network io collected"),
        Err(e) => println!("error collecting network io: {:?}", e),
    }
    match collect_disk_io(meter.clone()) {
        Ok(_) => println!("disk io collected"),
        Err(e) => println!("error collecting disk io: {:?}", e),
    }
    match collect_filesystem_usage(meter.clone()) {
        Ok(_) => println!("filesystem usage collected"),
        Err(e) => println!("error collecting filesystem usage: {:?}", e),
    }
    Ok(true)
}

// SYSTEM_CPU_UTILIZATION("system_cpu_time_seconds_total"),
fn collect_cpu_utilization(meter: Meter) -> Result<()> {
    let cpu_utilization_obs_counter = meter
        .f64_observable_gauge("system.cpu.utilization")
        .with_description("")
        .with_unit(Unit::new("1"))
        .init();
    match meter.register_callback(&[cpu_utilization_obs_counter.as_any()], move |observer| {
        let mut s =
            System::new_with_specifics(RefreshKind::new().with_cpu(CpuRefreshKind::everything()));
        let mut total_cpu_usage_vec: Vec<f32> = vec![0.0];
        let cpus = s.cpus();
        for cpu in cpus {
            println!("cpu destructure: {:?}", cpu); //todo: remove
            total_cpu_usage_vec.push(cpu.cpu_usage());
        }
        let attrs =
            vec![Key::new("system.cpu.logical_number").i64(total_cpu_usage_vec.len() as i64)];

        let result = calculate_average(&total_cpu_usage_vec);
        println!("total cpu usage: {}", result);
        observer.observe_f64(&cpu_utilization_obs_counter, result as f64, &attrs);
    }) {
        Ok(_) => println!("callback registered"),
        Err(e) => println!("error registering callback: {:?}", e),
    };
    Ok(())
}

fn calculate_average(numbers: &Vec<f32>) -> f64 {
    let sum: f32 = numbers.iter().sum();
    let count = numbers.len() as f64;
    let average = sum as f64 / count;
    average
}
// SYSTEM_MEMORY_USAGE("system_memory_usage_bytes"),
fn collect_memory_usage(meter: Meter) -> Result<()> {
    let memory_utilization_obs_counter = meter
        .f64_observable_up_down_counter("system.memory.usage")
        .with_description("Reports memory in use by state.")
        .with_unit(Unit::new("By"))
        .init();
    match meter.register_callback(
        &[memory_utilization_obs_counter.as_any()],
        move |observer| {
            let used_mem = System::new_all().used_memory();
            let attrs = vec![Key::new("state").string("used")];
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
        .f64_observable_gauge("system.cpu.load_average.15m")
        .with_description("Difference in system.cpu.time since the last measurement, divided by the elapsed time and number of logical CPUs.")
        .with_unit(Unit::new("1"))
        .init();
    match meter.register_callback(&[cpu_utilization_obs_counter.as_any()], move |observer| {
        let load_avg = System::load_average();
        let attrs = vec![];
        observer.observe_f64(
            &cpu_utilization_obs_counter,
            load_avg.fifteen as f64,
            &attrs,
        );
    }) {
        Ok(_) => println!("callback registered"),
        Err(e) => println!("error registering callback: {:?}", e),
    };
    Ok(())
}

// SYSTEM_NETWORK_IO("system_network_io_bytes_total"),
fn collect_network_io(meter: Meter) -> Result<()> {
    let network_io_obs_counter = meter
        .f64_observable_counter("system.network.io")
        .with_description("")
        .with_unit(Unit::new("By"))
        .init();

    match meter.register_callback(&[network_io_obs_counter.as_any()], move |observer| {
        let networks = Networks::new_with_refreshed_list();

        for (interface_name, network) in &networks {
            let total_transmitted_bytes = network.transmitted();
            let total_received_bytes = network.received();

            let attrs_transmit = vec![
                Key::new("direction").string("transmit"),
                Key::new("device").string(interface_name.to_owned()),
            ];
            observer.observe_f64(
                &network_io_obs_counter,
                total_transmitted_bytes as f64,
                &attrs_transmit,
            );

            let attrs_receive = vec![
                Key::new("direction").string("receive"),
                Key::new("device").string(interface_name.to_owned()),
            ];
            observer.observe_f64(
                &network_io_obs_counter,
                total_received_bytes as f64,
                &attrs_receive,
            );
        }
    }) {
        Ok(_) => println!("callback registered"),
        Err(e) => println!("error registering callback: {:?}", e),
    };

    Ok(())
}

// SYSTEM_DISK_IO("system_disk_io_bytes_total"),
fn collect_disk_io(meter: Meter) -> Result<()> {
    let disk_io_obs_counter = meter
        .f64_observable_counter("system.disk.io")
        .with_description("")
        .with_unit(Unit::new("By"))
        .init();
    match meter.register_callback(&[disk_io_obs_counter.as_any()], move |observer| {
        let s = System::new_all();
        let mut total_disk_usage_read_direction: u64 = 0;
        for (_pid, process) in s.processes() {
            total_disk_usage_read_direction += process.disk_usage().read_bytes;
        }
        let attrs = vec![Key::new("direction").string("read")];
        observer.observe_f64(
            &disk_io_obs_counter,
            total_disk_usage_read_direction as f64,
            &attrs,
        );

        let mut total_disk_usage_write_direction: u64 = 0;
        for (_pid, process) in s.processes() {
            total_disk_usage_write_direction += process.disk_usage().written_bytes;
        }
        let attrs = vec![Key::new("direction").string("write")];
        observer.observe_f64(
            &disk_io_obs_counter,
            total_disk_usage_write_direction as f64,
            &attrs,
        );
    }) {
        Ok(_) => println!("callback registered"),
        Err(e) => println!("error registering callback: {:?}", e),
    };
    Ok(())
}

// SYSTEM_FILESYSTEMS_USAGE("system_filesystem_usage_bytes");
fn collect_filesystem_usage(meter: Meter) -> Result<()> {
    let filesystem_usage_obs_counter = meter
        .f64_observable_up_down_counter("system.filesystem.usage")
        .with_description("")
        .with_unit(Unit::new("By"))
        .init();
    match meter.register_callback(&[filesystem_usage_obs_counter.as_any()], move |observer| {
        let disks = Disks::new_with_refreshed_list();
        let mut used_space: u64 = 0;
        for disk in disks.list() {
            used_space += disk.total_space() - disk.available_space();

            let mount_point = disk.mount_point().to_owned();
            let mut attrs = vec![];
            attrs.push(Key::new("state").string("used"));
            attrs.push(Key::new("mountpoint").string(mount_point.to_str().unwrap().to_owned()));
            observer.observe_f64(&filesystem_usage_obs_counter, used_space as f64, &attrs);
        }
    }) {
        Ok(_) => println!("callback registered"),
        Err(e) => println!("error registering callback: {:?}", e),
    };
    Ok(())
}
