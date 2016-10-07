extern crate isatty;
extern crate rand;
extern crate sfml;
#[macro_use (o, slog_log, slog_trace, slog_debug, slog_info, slog_warn, slog_error)]
extern crate slog;
extern crate slog_json;
#[macro_use]
extern crate slog_scope;
extern crate slog_stream;
extern crate slog_term;
extern crate tile_net;
extern crate time;

use std::thread;
use std::time::Duration;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::collections::BTreeMap;

use sfml::graphics::{CircleShape, Color, Font, RectangleShape, RenderTarget, RenderWindow, Shape,
                     Text, Transformable, Drawable, RenderStates};
use sfml::window::{ContextSettings, Key, VideoMode, event, window_style};
use sfml::system::{Clock, Time, Vector2f};
use sfml::audio::{Sound, SoundBuffer, SoundSource};
use tile_net::*;
use slog::DrainExt;

fn setup_logger() {
	let logger = if isatty::stderr_isatty() {
		slog::Logger::root(slog_term::streamer()
			                   .async()
			                   .stderr()
			                   .full()
			                   .use_utc_timestamp()
			                   .build()
			                   .ignore_err(),
		                   o![])
	} else {
		slog::Logger::root(slog_stream::stream(std::io::stderr(), slog_json::default()).fuse(),
		                   o![])
	};
	slog_scope::set_global_logger(logger);
}

fn main() {

	// implementing gjk, we need both vertices:
	#[derive(Clone, Copy)]
	struct Vec3(f32, f32, f32);
	struct Polygon(Vec<Vec3>);

	impl Vec3 {
		fn dot(&self, right: &Vec3) -> f32 {
			self.0*right.0+self.1*right.1+self.2*right.2
		}
	}

	impl std::ops::Sub for Vec3 {
		type Output = Vec3;

		fn sub(self, other: Vec3) -> Self::Output {
			Vec3(self.0 - other.0, self.1 - other.1, self.2 - other.2)
		}
	}

	use std::ops::Neg;
	impl<'a> std::ops::Neg for &'a Vec3 {
		type Output = Vec3;

		fn neg(self) -> Self::Output {
			Vec3(0.0 - self.0, 0.0 - self.1, 0.0 - self.2)
		}
	}

	fn farthest(a: Polygon, direction: &Vec3) -> Vec3 {
		let mut iter_a = a.0.iter();
		let mut curmax = iter_a.next().unwrap();
		let mut amax = curmax.dot(direction);
		for vector in iter_a {
			let curdot = vector.dot(direction);
			if curdot > amax {
				amax = curdot;
				curmax = vector;
			}
		}
		*curmax
	}

	fn support(a: Polygon, b: Polygon, direction: &Vec3) -> Vec3 {
		let far_a = farthest(a, direction);
		let far_b = farthest(b, &-direction);
		far_a - far_b
	}

	setup_logger();
	info!["Logger initialized"];

	let mut window = create_window();
	let net = create_tilenet();
	let mut tile = create_tile();
	let mut coller = Rects::new();
	let mut rarer = Rare::new(60);
	let gravity = 0.00981;

	'main: loop {
		if handle_events(&mut window) {
			break 'main;
		}

		let side_speed = 0.04;
		let vert_speed = 0.25;
		if Key::A.is_pressed() {
			coller.enqueue(Vector(-side_speed, 0.0));
		}
		if Key::D.is_pressed() {
			coller.enqueue(Vector(side_speed, 0.0));
		}

		rarer.run(|| println!("{:?}", coller));
		rarer.run(|| {
			net.collide_set(coller.tiles())
				.inspect(|x| info!("collides"; "col" => format!["{:?}", x]))
				.count();
		});

		let dy = coller.check_x();
		loop {
			let tiles = net.collide_set(coller.tiles());
			if !coller.resolve(tiles) {
				break;
			}
		}
		coller.uncheck_x(dy);

		if Key::W.is_pressed() {
			if coller.jmp {
				coller.set_speed(Vector(0.0, -vert_speed));
				coller.jmp = false;
			}
		}
		if Key::S.is_pressed() {
			coller.enqueue(Vector(0.0, vert_speed*100000.0));
		}

		rarer.run(|| info!["Current x speed"; "x" => coller.queued().1]);
		coller.enqueue(Vector(0.0, gravity));
		let mut any_col = false;
		loop {
			let down_speed = coller.queued().1;
			let if_break_jmp = down_speed > 1e-6;
			let tiles = net.collide_set(coller.tiles());
			if !coller.resolve(tiles) {
				if any_col == false {
					coller.jmp = false;
				}
				break;
			}
			if if_break_jmp {
				coller.jmp = true;
			}
			any_col = true;
		}

		rarer.run(|| info!["pos"; "pos" => format!["{:?}", coller.pos]]);

		window.clear(&Color::new_rgb(200, 2, 3));

		for i in net.view_all() {
			if let (&Some(_), col, row) = i {
				let col = col as f32;
				let row = row as f32;
				tile.set_position(&Vector2f::new(col * 100.0, row * 100.0));
				window.draw(&tile);
			}
		}

		window.draw(&coller);

		window.display();
	}
}

fn create_window() -> RenderWindow {
	let mut window = RenderWindow::new(VideoMode::new_init(800, 600, 42),
	                                   "Custom shape",
	                                   window_style::CLOSE,
	                                   &Default::default())
		.unwrap_or_else(|| {
			panic!("Could not create window");
		});
	window.set_framerate_limit(60);
	window
}

fn create_tilenet() -> tile_net::TileNet<usize> {
	//let mut net = tile_net::TileNet::new((1000, 1000));
	let mut net = tile_net::TileNet::sample();
	*net.get_mut((3, 2)).unwrap() = Some(0);
	(0..6)
		.map(|x| {
			*net.get_mut((0, x)).unwrap() = Some(0);
			*net.get_mut((7, x)).unwrap() = Some(0);
		})
		.count();
	(1..7)
		.map(|x| {
			*net.get_mut((x, 0)).unwrap() = Some(0);
		})
		.count();
	*net.get_mut((3, 2)).unwrap() = Some(0);
	net
}

fn create_block<'a>() -> RectangleShape<'a> {
	let mut block = RectangleShape::new().unwrap();
	block.set_size(&Vector2f::new(50.0, 50.0));
	block.set_fill_color(&Color::new_rgb(0, 0, 0));
	block.set_position2f(100.0, 100.0);
	block
}

fn create_tile<'a>() -> RectangleShape<'a> {
	let mut tile = RectangleShape::new().unwrap();
	tile.set_size(&Vector2f::new(100.0, 100.0));
	tile.set_fill_color(&Color::new_rgb(0, 200, 0));
	tile
}

fn handle_events(window: &mut RenderWindow) -> bool {
	for event in window.events() {
		match event {
			event::Closed => return true,
			event::KeyPressed { code, .. } => {
				match code {
					Key::Escape => return true,
					_ => {}
				}
			}
			_ => {}
		}
	}
	false
}

#[derive(Debug)]
struct Rects {
	pts: Vec<(f32, f32)>,
	pos: Vector,
	mov: Vector,
	jmp: bool,
	checking_x: bool,
}

impl Rects {
	fn new() -> Rects {
		Rects {
			pts: vec![(0.0, 0.0), (0.5, 0.0), (0.0, 0.5), (0.5, 0.5)],
			pos: Vector(1.0, 1.0),
			mov: Vector(0.0, 0.0),
			jmp: false,
			checking_x: false,
		}
	}

	fn check_x(&mut self) -> f32 {
		self.checking_x = true;
		let tmp = self.mov.1;
		self.mov = Vector(self.mov.0, 0.0);
		tmp
	}

	fn uncheck_x(&mut self, dy: f32) {
		self.checking_x = false;
		self.mov = Vector(self.mov.0, dy);
	}

	fn set_speed(&mut self, vec: Vector) {
		self.mov = vec;
	}

	fn get_pos(&self) -> Vector {
		self.pos
	}
}

impl Collable for Rects {
	fn points<'a>(&'a self) -> Points<'a> {
		Points::new(self.pos, &self.pts)
	}

	fn enqueue(&mut self, vector: Vector) {
		self.mov = self.mov + vector;
	}

	fn queued(&self) -> Vector {
		self.mov
	}

	fn resolve<'a, T, I>(&mut self, mut set: TileSet<'a, T, I>) -> bool
		where T: 'a,
		      I: Iterator<Item = (i32, i32)>
	{
		let mut mov = self.mov;
		self.mov = Vector(0.0, 0.0);
		if set.all(Option::is_none) {
			self.pos = self.pos + mov;
			self.mov = Vector(0.0, mov.1);
			false
		} else if mov.norm2sq() > 1e-6 {
			if self.checking_x {
				mov = Vector(mov.0 * 0.999, mov.1);
				self.mov = mov;
			} else {
				mov.scale(0.9);
				let gravity = 0.00981;
				if mov.norm2sq() > gravity {
					self.mov = Vector(mov.0, -mov.1);
				} else {
					self.mov = mov;
				}
			}
			true
		} else {
			false
		}
	}
}

impl Drawable for Rects {
	fn draw<R: RenderTarget>(&self, rt: &mut R, _: &mut RenderStates) {
		let mut block = create_block();
		block.set_position(&Vector2f::new(self.pos.0 * 100.0, self.pos.1 * 100.0));
		rt.draw(&block);
	}
}

struct Rare {
	count: usize,
	every: usize,
}

impl Rare {
	fn new(every: usize) -> Rare {
		Rare {
			count: 0,
			every: every,
		}
	}

	fn run<F: Fn()>(&mut self, function: F) {
		if self.count == self.every {
			self.count = 0;
			function();
		} else {
			self.count += 1;
		}
	}
}
