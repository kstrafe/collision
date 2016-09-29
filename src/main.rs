extern crate rand;
extern crate sfml;
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
use rand::{Rng, thread_rng};
use std::f32::consts::PI;
use std::env;
use tile_net::*;
use std::cell::{Cell, RefCell};

#[derive(Debug, Default)]
struct A {
	c: Option<Receiver<i32>>,
	d: Cell<i32>,
}

impl A {
	fn create(&mut self) -> Sender<i32> {
		let (tx, rx) = channel();
		self.c = Some(rx);
		tx
	}
}

#[derive(Debug, Default)]
struct B {
	c: Option<Sender<i32>>,
	i: Cell<usize>,
	f: Option<Sender<()>>,
}

impl B {
	fn setc(&mut self, sender: Sender<i32>) {
		self.c = Some(sender);
	}
	fn setf(&mut self, sender: Sender<()>) {
		self.f = Some(sender);
	}
}

#[derive(Clone, Debug, Default)]
struct C;

impl A {
	fn x(&self) {
		println!("A");
	}
	fn cycle(&mut self) {
		if let Some(ref x) = self.c {
			if let Ok(n) = x.try_recv() {
				self.d.set(n);
			}
		}
	}
}

impl B {
	fn x(&self) {
		println!("B");
	}
	fn cycle(&mut self) {
		if let Some(ref x) = self.c {
			self.i.set(self.i.get() + 1);
			x.send(rand::random());
		}

		if self.i.get() >= 1000000 {
			if let Some(ref x) = self.f {
				x.send(());
			}
		}
	}
}

impl C {
	fn x(&self) {
		println!("C");
	}
}

macro_rules! fsm {
	($($i:ident : $l:ty),*,) => {{
		fsm!($($i: $l),*)
	}};

	($($i:ident : $l:ty),*) => {{
		#[derive(Default)]
		struct State {
			$($i: $l),*
		}

		impl State {
			fn cycle(&mut self) {
				$(
					self.$i.cycle();
				)*
			}
		}

		State::default()
	}};
}

macro_rules! as_expr { ($x:expr) => ($x) }
macro_rules! prep_i {
	(($i:expr) ($($prev:tt)*) ($($cur:tt)*) ; $($rest:tt)*)  => {
		prep_i!(($i) ($($prev)* $i.$($cur)*;) () $($rest)*)
	};
	(($i:expr) ($($prev:tt)*) ($($cur:tt)*) $t:tt $($rest:tt)*)  => {
		prep_i!(($i) ($($prev)*) ($($cur)* $t) $($rest)*)
	};
	(($i:expr) ($($prev:tt)*) ())  => {
		as_expr!({$($prev)*})
	};
}

macro_rules! prep {
	($i:expr => $($t:tt)*) => {
		prep_i!(($i) () () $($t)*)
	};
}

fn main() {

	// let mut r = thread_rng();
	// for _ in 0..10 {
	// let mut mat = vec![];
	// for i in 0..(3200*1000)*(3) {
	// mat.push(/*r.next_f64()*/ i as f64);
	// }
	//
	//
	// let begin = time::now();
	// mat.sort_by(|a, b| a.partial_cmp(b).unwrap());
	// let end = time::now();
	// let diff = end - begin;
	// println!("{:?}", diff);
	// }
	// return;
	//

	// Three types of messages
	// cycle | Queue for next cycle using obj.send(...)
	// direct | Call a method directly using Fn(x) -> (x)
	// async | Queue to a thread using obj.send(...)

	// cycle:
	let mut fsm = fsm! {
		audio: A,
		some: B,
		// video: login.logState, login.qListen;
	};

	fsm.some.setc(fsm.audio.create());
	let (tx, rx) = channel();
	fsm.some.setf(tx);

	let begin = time::now();
	loop {
		fsm.cycle();
		if let Ok(()) = rx.try_recv() {
			break;
		}
	}
	let end = time::now();

	println!("{:?}", end - begin);
	println!("{:?}", fsm.audio);

	let mut rr: i32 = 0;
	let begin = time::now();
	for _ in 0..1000000 {
		rr = rand::random();
	}
	let end = time::now();
	println!("{:?}", end - begin);
	println!("{:?}", rr);

	// Ideally:
	//
	// let mut fsm = fsm! {
	// ...
	// };
	//
	// con!(fsm,
	// audio control <=> interface,
	// );
	//
	//

	let mut window = create_window();
	let mut net = create_tilenet();
	let mut tile = create_tile();
	let mut coller = Rects::new();
	let mut rarer = Rare::new(60);
	let mut gravity = 0.00981;
	let mut hit_ground = false;

	'main: loop {
		if handle_events(&mut window) {
			break 'main;
		}


		let mut uppressed = false;
		let side_speed = 0.02;
		let vert_speed = 0.2;
		if Key::Left.is_pressed() {
			coller.enqueue(Vector(-side_speed, 0.0));
		}
		if Key::Right.is_pressed() {
			coller.enqueue(Vector(side_speed, 0.0));
		}

		rarer.run(|| println!("{:?}", coller));
		rarer.run(|| {
			net.collide_set(coller.tiles())
				.inspect(|x| println!("{:?}", x))
				.count();
		});

		loop {
			let tiles = net.collide_set(coller.tiles());
			if !coller.resolve(tiles) {
				break;
			}
		}

		if Key::Up.is_pressed() {
			if hit_ground {
				coller.enqueue(Vector(0.0, -vert_speed));
				uppressed = true;
				hit_ground = false;
			}
		}
		if Key::Down.is_pressed() {
			coller.enqueue(Vector(0.0, vert_speed));
		}

		if !uppressed {
			coller.enqueue(Vector(0.0, gravity));
		}
		loop {
			let tiles = net.collide_set(coller.tiles());
			if !coller.resolve(tiles) {
				break;
			}
			hit_ground = true;
		}

		window.clear(&Color::new_rgb(200, 2, 3));
		for i in net.view_box((0, 10, 0, 10)) {
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
	let mut net = tile_net::TileNet::sample();
	*net.get_mut((3, 2)).unwrap() = Some(0);
	(0..6)
		.map(|x| {
			*net.get_mut((0, x)).unwrap() = Some(0);
			*net.get_mut((7, x)).unwrap() = Some(0);
		})
		.count();
	*net.get_mut((3, 2)).unwrap() = Some(0);
	net
}

fn create_block<'a>() -> RectangleShape<'a> {
	let mut block = RectangleShape::new().unwrap();
	block.set_size(&Vector2f::new(50.0, 50.0));
	block.set_fill_color(&Color::new_rgb(0, 0, 0));
	block.set_position2f(100.0, 0.0);
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
}

impl Rects {
	fn new() -> Rects {
		Rects {
			pts: vec![(0.0, 0.0), (0.5, 0.0), (0.0, 0.5), (0.5, 0.5)],
			pos: Vector(1.0, 0.0),
			mov: Vector(0.0, 0.0),
		}
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
			mov.scale(0.99);
			self.mov = mov;
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
