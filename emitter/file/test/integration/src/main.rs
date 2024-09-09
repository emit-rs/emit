/*!
A simple integration test of emitting events to rolling files.
*/

fn main() {
    let events_per_iteration = 10_000;
    let iterations = 7;

    if std::path::Path::new("./logs").exists() {
        std::fs::remove_dir_all("./logs").unwrap();
    }

    // Configure emit to write to rolling files
    let _ = emit::setup().emit_to(emit_term::stdout()).init_internal();

    for i in 0..iterations {
        let slot = emit::runtime::AmbientSlot::new();

        let mut reporter = emit::metric::Reporter::new();

        let rt = emit::setup()
            .emit_to({
                let emitter = emit_file::set("./logs/test.log")
                    .reuse_files(i % 2 == 0)
                    .spawn();

                reporter.add_source(emitter.metric_source());

                emitter
            })
            .init_slot(&slot);

        // Write our events
        for i in 0..events_per_iteration {
            emit::emit!(rt: slot.get(), "Event #{i}");
        }

        // Wait for writing to complete
        rt.blocking_flush(std::time::Duration::from_secs(10));

        reporter.emit_metrics(emit_term::stdout());
    }

    // Ensure all events were written
    let mut read_count = 0;
    for f in std::fs::read_dir("./logs").unwrap() {
        let f = f.unwrap();

        let contents = std::fs::read_to_string(f.path()).unwrap();

        for line in contents.lines() {
            if line.trim().is_empty() {
                continue;
            }

            // Ensure the line is valid JSON
            let _: serde_json::Value = serde_json::from_str(line).unwrap();
            read_count += 1;
        }
    }

    assert_eq!(
        events_per_iteration * iterations,
        read_count,
        "unexpected total event count"
    );
}
