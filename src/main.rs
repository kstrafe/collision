extern crate rand;
extern crate sfml;
extern crate tile_net;

use sfml::graphics::{CircleShape, Color, Font, RectangleShape, RenderTarget, RenderWindow, Shape,
                     Text, Transformable};
use sfml::window::{ContextSettings, Key, VideoMode, event, window_style};
use sfml::system::{Clock, Time, Vector2f};
use sfml::audio::{Sound, SoundBuffer, SoundSource};
use rand::{Rng, thread_rng};
use std::f32::consts::PI;
use std::env;
use tile_net::*;

fn main() {
	let mut window = match RenderWindow::new(VideoMode::new_init(800, 600, 42),
		"Custom shape",
		window_style::CLOSE,
		&Default::default()) {
		Some(window) => window,
		None => panic!("SHIT"),
	};

	let mut block = RectangleShape::new().unwrap();
	block.set_size(&Vector2f::new(100.0, 100.0));
	block.set_fill_color(&Color::new_rgb(0, 0, 0));

	let net = tile_net::TileNet::sample();

	window.set_framerate_limit(60);

	let per = 60;
	let mut cur = 0;

	'main: loop {
		for event in window.events() {
			match event {
				event::Closed => break 'main,
				event::KeyPressed {code, ..} => {
					match code {
						Key::Escape => break 'main,
						_ => {},
					}
				}
				_ => {},
			}
		}

		let oldpos = block.get_position();
		if Key::Up.is_pressed() {
			block.move2f(0.0, -1.0);
		}
		if Key::Down.is_pressed() {
			block.move2f(0.0, 1.0);
		}
		if Key::Left.is_pressed() {
			block.move2f(-1.0, 0.0);
		}
		if Key::Right.is_pressed() {
			block.move2f(1.0, 0.0);
		}

		let rect = block.get_global_bounds();
		let left_bottom = Point(rect.left/100.0, (rect.top + rect.height)/100.0);
		let right_bottom = Point((rect.left + rect.width)/100.0, (rect.top + rect.height)/100.0);
		let line = Line(left_bottom, right_bottom);
		if !net.collide_set(line.supercover()).all(|x| x == &None) {
			block.set_position(&oldpos);
		}

		window.clear(&Color::new_rgb(200, 2, 3));
		window.draw(&block);
		window.display();
	}
}
