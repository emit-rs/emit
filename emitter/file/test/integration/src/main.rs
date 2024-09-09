/*!
A simple integration test of emitting events to rolling files.
*/

fn main() {
    if std::path::Path::new("./logs").exists() {
        std::fs::remove_dir_all("./logs").unwrap();
    }

    // Configure emit to write to rolling files
    let _ = emit::setup().emit_to(emit_term::stdout()).init_internal();

    let mut reporter = emit::metric::Reporter::new();

    let rt = emit::setup()
        .emit_to({
            let emitter = emit_file::set("./logs/test.log").spawn();

            reporter.add_source(emitter.metric_source());

            emitter
        })
        .init();

    let expected_count = 10_000;

    // Write our events
    for i in 0..expected_count {
        emit::emit!("Event #{i}");
    }

    // Wait for writing to complete
    rt.blocking_flush(std::time::Duration::from_secs(10));

    reporter.emit_metrics(emit_term::stdout());

    // Ensure all events were written
    let mut read_count = 0;
    for f in std::fs::read_dir("./logs").unwrap() {
        let f = f.unwrap();

        let contents = std::fs::read_to_string(f.path()).unwrap();

        for line in contents.lines() {
            // Ensure the line is valid JSON
            let _: serde_json::Value = serde_json::from_str(line).unwrap();
            read_count += 1;
        }
    }

    assert_eq!(expected_count, read_count, "unexpected total event count");
}
