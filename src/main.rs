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
                     Text, Transformable, Drawable, RenderStates, View};
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
			self.0 * right.0 + self.1 * right.1 + self.2 * right.2
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
	let mut coller2 = RectsWhite::new();
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

		let dy = coller2.check_x();
		loop {
			let tiles = net.collide_set(coller2.tiles());
			if !coller2.resolve(tiles) {
				break;
			}
		}
		coller2.uncheck_x(dy);

		if Key::W.is_pressed() {
			if coller.jmp {
				coller.set_speed(Vector(0.0, -vert_speed));
				coller.jmp = false;
			}
		}
		if Key::S.is_pressed() {
			coller.enqueue(Vector(0.0, vert_speed * 100000.0));
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

		coller2.enqueue(Vector(0.0, gravity));
		let mut any_col = false;
		loop {
			let down_speed = coller2.queued().1;
			let if_break_jmp = down_speed > 1e-6;
			let tiles = net.collide_set(coller2.tiles());
			if !coller2.resolve(tiles) {
				if any_col == false {
					coller2.jmp = false;
				}
				break;
			}
			if if_break_jmp {
				coller2.jmp = true;
			}
			any_col = true;
		}
		rarer.run(|| info!["pos"; "pos" => format!["{:?}", coller2.pos]]);

		window.clear(&Color::new_rgb(255, 255, 255));

		let mut view = View::new().unwrap();

		let pos = coller.get_pos();
		let xbegin = pos.0 as usize;
		let ybegin = pos.1 as usize;

		view.set_center(&Vector2f::new(pos.0 * 10.0, pos.1 * 10.0));
		window.set_view(&view);

		for i in net.view_center((xbegin, ybegin), (120usize, 60usize)) {
			if let (&1, col, row) = i {
				let col = col as f32;
				let row = row as f32;
				tile.set_position(&Vector2f::new(col * 10.0, row * 10.0));
				window.draw(&tile);
			}
		}

		window.draw(&coller2);
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
	let mut net: TileNet<usize> = tile_net::TileNet::new((1000, 1000));
	*net.get_mut((3, 2)).unwrap() = 1;
	(0..1000)
		.map(|x| {
			*net.get_mut((0, x)).unwrap() = 1;
			*net.get_mut((999, x)).unwrap() = 1;
		})
		.count();
	(0..1000)
		.map(|x| {
			*net.get_mut((x, 0)).unwrap() = 1;
			for i in 800..980 {
				*net.get_mut((x, i)).unwrap() = 1;
			}
			*net.get_mut((x, 999)).unwrap() = 1;
		})
		.count();
	*net.get_mut((3, 2)).unwrap() = 1;
	net
}

fn create_block<'a>() -> RectangleShape<'a> {
	let mut block = RectangleShape::new().unwrap();
	block.set_size(&Vector2f::new(10.0, 10.0));
	block.set_fill_color(&Color::new_rgb(0, 0, 0));
	block.set_position2f(900.0, 900.0);
	block
}

fn create_tile<'a>() -> RectangleShape<'a> {
	let mut tile = RectangleShape::new().unwrap();
	tile.set_size(&Vector2f::new(10.0, 10.0));
	tile.set_fill_color(&Color::new_rgb(0, 0, 0));
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
struct RectsWhite {
	pts: Vec<(f32, f32)>,
	pos: Vector,
	mov: Vector,
	jmp: bool,
	checking_x: bool,
}

impl RectsWhite {
	fn new() -> RectsWhite {
		RectsWhite {
			pts: vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)],
			pos: Vector(2.0, 970.0),
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

impl Collable<usize> for RectsWhite {
	fn points<'a>(&'a self) -> Points<'a> {
		Points::new(self.pos, &self.pts)
	}

	fn enqueue(&mut self, vector: Vector) {
		self.mov = self.mov + vector;
	}

	fn queued(&self) -> Vector {
		self.mov
	}

	fn resolve<'a, I>(&mut self, mut set: TileSet<'a, usize, I>) -> bool
		where I: Iterator<Item = (i32, i32)>
	{
		let mut mov = self.mov;
		self.mov = Vector(0.0, 0.0);
		if set.all(|x| *x == 1) {
			self.pos = self.pos + mov;
			self.mov = Vector(0.0, mov.1);
			false
		} else if mov.norm2sq() > 1e-6 {
			if self.checking_x {
				mov = Vector(mov.0 * 0.5, mov.1);
				self.mov = mov;
			} else {
				mov.scale(0.5);
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

impl Drawable for RectsWhite {
	fn draw<R: RenderTarget>(&self, rt: &mut R, _: &mut RenderStates) {
		let mut block = create_block();
		block.set_fill_color(&Color::new_rgb(255, 255, 255));
		block.set_position(&Vector2f::new(self.pos.0 * 10.0, self.pos.1 * 10.0));
		rt.draw(&block);
	}
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
			pts: vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)],
			pos: Vector(2.0, 990.0),
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

impl Collable<usize> for Rects {
	fn points<'a>(&'a self) -> Points<'a> {
		Points::new(self.pos, &self.pts)
	}

	fn enqueue(&mut self, vector: Vector) {
		self.mov = self.mov + vector;
	}

	fn queued(&self) -> Vector {
		self.mov
	}

	fn resolve<'a, I>(&mut self, mut set: TileSet<'a, usize, I>) -> bool
		where I: Iterator<Item = (i32, i32)>
	{
		let mut mov = self.mov;
		self.mov = Vector(0.0, 0.0);
		if set.all(|x| *x == 0usize) {
			self.pos = self.pos + mov;
			self.mov = Vector(0.0, mov.1);
			false
		} else if mov.norm2sq() > 1e-6 {
			if self.checking_x {
				mov = Vector(mov.0 * 0.59, mov.1);
				self.mov = mov;
			} else {
				mov.scale(0.6);
				let gravity = 0.00981;
				self.mov = mov;
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
		block.set_position(&Vector2f::new(self.pos.0 * 10.0, self.pos.1 * 10.0));
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
