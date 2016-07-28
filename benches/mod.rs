#![feature(test)]

extern crate test;
extern crate emit;
extern crate chrono;

use std::io::Cursor;
use std::collections::BTreeMap;
use test::Bencher;
use chrono::{ UTC, TimeZone };
use emit::{ LogLevel, templates, events };
use emit::events::{ IntoValue, SanitiserVisitor };
use emit::formatters::WriteEvent;
use emit::formatters::json::JsonSanitiser;

fn some_event() -> events::Event<'static> {
	let ts = UTC.ymd(2014, 7, 8).and_hms(9, 10, 11);
    let mt = templates::MessageTemplate::new("Hello, {name}");
    let mut props = BTreeMap::new();
    props.insert("name", "Alice".into_value());
    props.insert("data", vec!["a", "b", "c"].into_value());
    events::Event::new(ts, LogLevel::Info, mt, props)
}

#[bench]
pub fn template_repl(b: &mut Bencher) {
	let template = templates::repl::MessageTemplateRepl::new("Some value A: {A} And some other value: {Bert} There are no more values to parse");
	let mut map = BTreeMap::new();

    map.insert("A", "value1".into_value());
    map.insert("Bert", "value2".into_value());

	b.iter(|| {
		test::black_box(template.replace(&map));
	});
}

#[bench]
pub fn template_repl_new(b: &mut Bencher) {
	b.iter(|| {
		test::black_box(templates::repl::MessageTemplateRepl::new("Some value A: {A} And some other value: {Bert} There are no more values to parse"));
	});
}

#[bench]
pub fn template_from_format(b: &mut Bencher) {
	b.iter(|| {
		test::black_box(templates::MessageTemplate::from_format("Some value A: {} And some other value: {} There are no more values to parse", &vec!["A", "Bert"]));
	});
}

#[bench]
pub fn template_new(b: &mut Bencher) {
	b.iter(|| {
		test::black_box(templates::MessageTemplate::new("Some value A: {A} And some other value: {Bert} There are no more values to parse"));
	});
}

#[bench]
pub fn format_json(b: &mut Bencher) {
	let evt = some_event();
	let fmtr = emit::formatters::json::JsonFormatter::new();
	b.iter(|| {
		let mut json = Cursor::new(Vec::new());
		fmtr.write_event(&evt, &mut json).unwrap();
		test::black_box(json);
	});
}

#[bench]
pub fn format_json_sized(b: &mut Bencher) {
	let evt = some_event();
	let fmtr = emit::formatters::json::JsonFormatter::new();

	let len = {
		let mut json = Cursor::new(Vec::new());
		fmtr.write_event(&evt, &mut json).unwrap();

		json.into_inner().len()
	};

	b.iter(|| {
		let mut json = Cursor::new(Vec::with_capacity(len));
		fmtr.write_event(&evt, &mut json).unwrap();
		test::black_box(json);
	});
}

#[bench]
pub fn format_json_rendered(b: &mut Bencher) {
	let evt = some_event();
	let fmtr = emit::formatters::json::RenderedJsonFormatter::new();
	b.iter(|| {
		let mut json = Cursor::new(Vec::new());
		fmtr.write_event(&evt, &mut json).unwrap();
		test::black_box(json);
	});
}

#[bench]
pub fn format_raw(b: &mut Bencher) {
	let evt = some_event();
	let fmtr = emit::formatters::raw::RawFormatter::new();
	b.iter(|| {
		let mut json = Cursor::new(Vec::new());
		fmtr.write_event(&evt, &mut json).unwrap();
		test::black_box(json);
	});
}

#[bench]
pub fn format_text(b: &mut Bencher) {
	let evt = some_event();
	let fmtr = emit::formatters::text::PlainTextFormatter::new();
	b.iter(|| {
		let mut json = Cursor::new(Vec::new());
		fmtr.write_event(&evt, &mut json).unwrap();
		test::black_box(json);
	});
}

#[bench]
pub fn str_to_value(b: &mut Bencher) {
	let value = "teststring";
	b.iter(|| {
		let v: events::Value = value.into_value();
		test::black_box(v);
	});
}

#[bench]
pub fn vec_to_value(b: &mut Bencher) {
	b.iter(|| {
		let v: events::Value = vec!["a","b","c"].into_value();
		test::black_box(v);
	});
}

#[bench]
pub fn i64_to_value(b: &mut Bencher) {
	let value = 4i64;
	b.iter(|| {
		let v: events::Value = value.into_value();
		test::black_box(v);
	});
}

#[bench]
pub fn i8_to_value(b: &mut Bencher) {
	let value = 4i8;
	b.iter(|| {
		let v: events::Value = value.into_value();
		test::black_box(v);
	});
}

#[bench]
pub fn i16_to_value(b: &mut Bencher) {
	let value = 4i16;
	b.iter(|| {
		let v: events::Value = value.into_value();
		test::black_box(v);
	});
}

#[bench]
pub fn i32_to_value(b: &mut Bencher) {
	let value = 4i32;
	b.iter(|| {
		let v: events::Value = value.into_value();
		test::black_box(v);
	});
}

#[bench]
pub fn u64_to_value(b: &mut Bencher) {
	let value = 4u64;
	b.iter(|| {
		let v: events::Value = value.into_value();
		test::black_box(v);
	});
}

#[bench]
pub fn u8_to_value(b: &mut Bencher) {
	let value = 4u8;
	b.iter(|| {
		let v: events::Value = value.into_value();
		test::black_box(v);
	});
}

#[bench]
pub fn u16_to_value(b: &mut Bencher) {
	let value = 4u16;
	b.iter(|| {
		let v: events::Value = value.into_value();
		test::black_box(v);
	});
}

#[bench]
pub fn u32_to_value(b: &mut Bencher) {
	let value = 4u32;
	b.iter(|| {
		let v: events::Value = value.into_value();
		test::black_box(v);
	});
}

#[bench]
pub fn f64_to_value(b: &mut Bencher) {
	let value = 4f64;
	b.iter(|| {
		let v: events::Value = value.into_value();
		test::black_box(v);
	});
}

#[bench]
pub fn f32_to_value(b: &mut Bencher) {
	let value = 4f32;
	b.iter(|| {
		let v: events::Value = value.into_value();
		test::black_box(v);
	});
}

#[bench]
pub fn str_value_to_json(b: &mut Bencher) {
	let sanitiser = JsonSanitiser::sanitiser();
	b.iter(|| {
		let j = JsonSanitiser::visit_str(&sanitiser, "teststring");
		test::black_box(j);
	});
}

#[bench]
pub fn vec_value_to_json(b: &mut Bencher) {
	let sanitiser = JsonSanitiser::sanitiser();
	let v = vec![
		"a".into_value(),
		"b".into_value(),
		"c".into_value()
	];

	b.iter(|| {
		let j = JsonSanitiser::visit_vec(&sanitiser, &v);
		test::black_box(j);
	});
}

#[bench]
pub fn i64_value_to_json(b: &mut Bencher) {
	let v = 4i64;
	let sanitiser = JsonSanitiser::sanitiser();
	b.iter(|| {
		let j = JsonSanitiser::visit_i64(&sanitiser, &v);
		test::black_box(j);
	});
}

#[bench]
pub fn u64_value_to_json(b: &mut Bencher) {
	let v = 4u64;
	let sanitiser = JsonSanitiser::sanitiser();
	b.iter(|| {
		let j = JsonSanitiser::visit_u64(&sanitiser, &v);
		test::black_box(j);
	});
}

#[bench]
pub fn f64_value_to_json(b: &mut Bencher) {
	let v = 4f64;
	let sanitiser = JsonSanitiser::sanitiser();
	b.iter(|| {
		let j = JsonSanitiser::visit_f64(&sanitiser, &v);
		test::black_box(j);
	});
}