#![feature(test)]

extern crate test;
extern crate emit;

use std::collections::BTreeMap;
use test::Bencher;
use emit::templates;

#[bench]
pub fn template_repl(b: &mut Bencher) {
	let template = templates::repl::MessageTemplateRepl::new("Some value A: {A} And some other value: {Bert} There are no more values to parse");
	let mut map = BTreeMap::new();

    map.insert("A", "value1");
    map.insert("Bert", "value2");

	b.iter(|| {
		template.replace(&map)
	});
}

#[bench]
pub fn template_from_format(b: &mut Bencher) {
	b.iter(|| {
		templates::MessageTemplate::from_format("Some value A: {} And some other value: {} There are no more values to parse", &vec!["A", "Bert"])
	});
}

#[bench]
pub fn template_from_new(b: &mut Bencher) {
	b.iter(|| {
		templates::MessageTemplate::new("Some value A: {A} And some other value: {Bert} There are no more values to parse")
	});
}

#[bench]
pub fn template_fmt(b: &mut Bencher) {
	b.iter(|| {
		format!("Some value A: {A} And some other value: {Bert} There are no more values to parse", A="value1", Bert="value2")
	});
}