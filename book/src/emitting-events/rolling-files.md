# Emitting to rolling files

- Use `emit_file` to emit events to rolling files in line-delimited JSON.
- Format can be customized.
- Can roll files by minute, hour, day.
- Can limit size based on file size and count.
- Recovers from IO errors.
