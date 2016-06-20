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

fn main() {
	let mut window = match RenderWindow::new(VideoMode::new_init(800, 600, 42),
		"Custom shape",
		window_style::CLOSE,
		&Default::default()) {
		Some(window) => window,
		None => panic!("SHIT"),
	};

	window.set_framerate_limit(60);
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
		window.clear(&Color::new_rgb(200, 2, 3));
		window.display();
	}
}
