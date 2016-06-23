extern crate rand;
extern crate sfml;
extern crate tile_net;

use std::thread;
use std::time::Duration;
use std::sync::mpsc::{channel, Sender};

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
	block.set_position2f(100.0, 0.0);
	let mut tile = RectangleShape::new().unwrap();
	tile.set_size(&Vector2f::new(100.0, 100.0));
	tile.set_fill_color(&Color::new_rgb(0, 200, 0));

	let mut net = tile_net::TileNet::sample();
	*net.get_mut((3, 2)).unwrap() = Some(0);
	(0..6)
		.map(|x| {
			*net.get_mut((0, x)).unwrap() = Some(0);
		})
		.count();
	*net.get_mut((3, 2)).unwrap() = Some(0);

	window.set_framerate_limit(60);

	let per = 60;
	let mut cur = 0;
	let mut speed = 10000000000.0;
	let mut gravity = 0.0981;

	'main: loop {
		for event in window.events() {
			match event {
				event::Closed => break 'main,
				event::KeyPressed { code, .. } => {
					match code {
						Key::Escape => break 'main,
						_ => {}
					}
				}
				_ => {}
			}
		}

		println!("{:?}", block.get_position());

		let side_speed = 2.0;
		let oldpos = block.get_position();
		if Key::Up.is_pressed() {
			speed -= 0.5;
			block.move2f(0.0, -1.0);
		}
		if Key::Down.is_pressed() {
			block.move2f(0.0, 1.0);
		}
		if Key::Left.is_pressed() {
			block.move2f(-side_speed, 0.0);
		}
		if Key::Right.is_pressed() {
			block.move2f(side_speed, 0.0);
		}
		gravity = 0.0;

		let rect = block.get_global_bounds();
		let left_bottom = Point(rect.left / 100.0, (rect.top + rect.height) / 100.0);
		block.move2f(0.0, speed);
		let rect = block.get_global_bounds();
		let right_bottom = Point(rect.left / 100.0, (rect.top + rect.height) / 100.0);
		let line = Line(left_bottom, right_bottom);
		if !net.collide_set(line.supercover()).all(|x| x == &None) {
			println!("Collided");
			gravity = 0.0;
			block.set_position(&oldpos);
			speed = 0.0;
		}

		gravity = 0.0981;
		speed += gravity;
		let oldpos = block.get_position();

		let rect = block.get_global_bounds();
		let left_bottom = Point(rect.left / 100.0, (rect.top + rect.height) / 100.0);
		block.move2f(0.0, speed);
		let rect = block.get_global_bounds();
		let right_bottom = Point(rect.left / 100.0, (rect.top + rect.height) / 100.0);
		let line = Line(left_bottom, right_bottom);
		if !net.collide_set(line.supercover()).all(|x| x == &None) {
			println!("Collided");
			gravity = 0.0;
			block.set_position(&oldpos);
			speed = 0.0;
		}

		window.clear(&Color::new_rgb(200, 2, 3));
		window.draw(&block);
		for (index, i) in net.view_box((0, 10, 0, 10)).enumerate() {
			if let &Some(_) = i {
				let col = (index % 10) as f32;
				let row = (index / 10) as f32;
				tile.set_position(&Vector2f::new(col * 100.0, row * 100.0));
				window.draw(&tile);
			}
		}
		window.display();
	}
}
