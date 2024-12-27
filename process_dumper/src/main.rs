use minidump_writer::app_memory::AppMemory;

fn main() -> anyhow::Result<()> {
    let mut minidump_file = std::fs::File::create("example_dump.mdmp")?;

    let pid = 164833;
    let maps = proc_maps::get_process_maps(pid.into())?;

    let mut app_memory = vec![];
    for m in maps {
        if m.filename()
            .as_ref()
            .and_then(|p| p.to_str())
            .map(|p| !["[vvar]", "[vsyscall]"].contains(&p))
            .unwrap_or(true)
        {
            app_memory.push(AppMemory {
                ptr: m.start(),
                length: m.size(),
            });
        }
    }

    minidump_writer::minidump_writer::MinidumpWriter::new(pid.into(), pid.into())
        .set_app_memory(app_memory)
        .dump(&mut minidump_file)?;

    Ok(())
}
