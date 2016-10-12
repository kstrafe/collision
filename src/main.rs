#![cfg_attr(feature = "dev", allow(unstable_features))]
#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]
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

use sfml::graphics::{Color, RectangleShape, RenderTarget, RenderWindow, Shape, Transformable,
                     Drawable, RenderStates, View};
use sfml::window::{Key, VideoMode, event, window_style};
use sfml::system::Vector2f;
use tile_net::*;
use slog::DrainExt;

use std::ops::{Neg, Sub};

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

#[derive(Clone, Copy, Debug, Default)]
struct Vec3(f32, f32, f32);
impl Vec3 {
	fn dot(&self, right: Vec3) -> f32 {
		self.0 * right.0 + self.1 * right.1 + self.2 * right.2
	}
}
impl Sub for Vec3 {
	type Output = Vec3;
	fn sub(self, right: Vec3) -> Self::Output {
		Vec3(self.0 - right.0, self.1 - right.1, self.2 - right.2)
	}
}
impl Neg for Vec3 {
	type Output = Vec3;
	fn neg(self) -> Self::Output {
		Vec3(-self.0, -self.1, -self.2)
	}
}

fn farthest(vertices: &[Vec3], direction: Vec3) -> Vec3 {
	let mut max: Option<f32> = None;
	let mut max_vertex = Vec3::default();
	for vertex in vertices {
		let current = vertex.dot(direction);
		if let Some(value) = max {
			if current > value {
				max = Some(current);
				max_vertex = *vertex;
			}
		} else {
			max = Some(current);
			max_vertex = *vertex;
		}
	}
	max_vertex
}

fn support(vertices_a: &[Vec3], vertices_b: &[Vec3], direction: Vec3) -> Vec3 {
	farthest(vertices_a, direction) - farthest(vertices_b, -direction)
}

/// The BGJK algorithm
///
/// The Boolean-GJK algorithm gives us the answer to the question:
/// "do these convex hulls intersect?"
/// This algorithm takes two hulls. The ordering of the points is not
/// important. All points are assumed to be on the surface of the hull.
/// Having interior points should not affect the qualitative result of
/// the algorithm, but may cause slight (very minor) degradation in
/// performance. The algorithm is O(n+m), where n and m are the amount
/// of points in hull1 and hull2 respectively.
fn bgjk(hull1: &[Vec3], hull2: &[Vec3]) -> bool {
	let mut sp = Vec3(1.0, 1.0, 1.0);
	let mut dp = Vec3::default();
	let (mut ap, mut bp, mut cp);

	cp = support(hull1, hull2, sp);
	sp = -cp;
	bp = support(hull1, hull2, sp);
	if bp.dot(sp) < 0.0 {
		return false;
	}
	sp = dcross3(cp - bp, -bp);
	let mut w = 2;

	loop {
		ap = support(hull1, hull2, sp);
		if ap.dot(sp) < 0.0 {
			return false;
		} else if simplex(&mut ap, &mut bp, &mut cp, &mut dp, &mut sp, &mut w) {
			return true;
		}
	}
}

fn simplex(ap: &mut Vec3,
           bp: &mut Vec3,
           cp: &mut Vec3,
           dp: &mut Vec3,
           sp: &mut Vec3,
           w: &mut i32)
           -> bool {
	let ao = -*ap;
	let mut ab = *bp - *ap;
	let mut ac = *cp - *ap;
	let mut abc = cross(ab, ac);
	match *w {
		2 => {
			let ab_abc = cross(ab, abc);
			if ab_abc.dot(ao) > 0.0 {
				*cp = *bp;
				*bp = *ap;
				*sp = dcross3(ab, ao);
			} else {
				let abc_ac = cross(abc, ac);
				if abc_ac.dot(ao) > 0.0 {
					*bp = *ap;
					*sp = dcross3(ac, ao);
				} else {
					if abc.dot(ao) > 0.0 {
						*dp = *cp;
						*cp = *bp;
						*bp = *ap;
						*sp = abc;
					} else {
						*dp = *bp;
						*bp = *ap;
						*sp = -abc;
					}
					*w = 3;
				}
			}
			false
		}
		3 => {
			macro_rules! check_tetrahedron {
				() => { check_tetra(Tetra(ap, bp, cp, dp), sp, w, ao, ab, ac, abc); };
			};
			if abc.dot(ao) > 0.0 {
				check_tetrahedron![];;
				false
			} else {
				let ad = *dp - *ap;
				let acd = cross(ac, ad);
				if acd.dot(ao) > 0.0 {
					*bp = *cp;
					*cp = *dp;
					ab = ac;
					ac = ad;
					abc = acd;
					check_tetrahedron![];;
					false
				} else {
					let adb = cross(ad, ab);
					if adb.dot(ao) > 0.0 {
						*cp = *bp;
						*bp = *dp;
						ac = ab;
						ab = ad;
						abc = adb;
						check_tetrahedron![];;
						false
					} else {
						true
					}
				}
			}
		}
		_ => false,
	}
}

struct Tetra<'a>(&'a mut Vec3, &'a mut Vec3, &'a mut Vec3, &'a mut Vec3);

fn check_tetra(te: Tetra, sp: &mut Vec3, w: &mut i32, ao: Vec3, ab: Vec3, ac: Vec3, abc: Vec3) {
	let ab_abc = cross(ab, abc);
	if ab_abc.dot(ao) > 0.0 {
		*te.2 = *te.1;
		*te.1 = *te.0;
		*sp = dcross3(ab, ao);
		*w = 2;
	} else {
		let acp = cross(abc, ac);
		if acp.dot(ao) > 0.0 {
			*te.1 = *te.0;
			*sp = dcross3(ac, ao);
			*w = 2;
		} else {
			*te.3 = *te.2;
			*te.2 = *te.1;
			*te.1 = *te.0;
			*sp = abc;
			*w = 3;
		}
	}
}

fn cross(a: Vec3, b: Vec3) -> Vec3 {
	Vec3(a.1 * b.2 - a.2 * b.1,
	     a.2 * b.0 - a.0 * b.2,
	     a.0 * b.1 - a.1 * b.0)
}

fn cross3(a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
	cross(cross(a, b), c)
}

fn dcross3(a: Vec3, b: Vec3) -> Vec3 {
	cross3(a, b, a)
}

mod tests {

	use std::f32;
	use super::{Vec3, bgjk};

	macro_rules! pts {
		($($e:expr),*) => {
			vec![$(
				Vec3($e.0, $e.1, $e.2)
			),*]
		};
	}

	#[test]
	fn square1() {
		let cube1 = pts![(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (1.0, 1.0, 0.0)];
		let cube2 = pts![(-2.0, 0.0, 0.0), (-3.0, 0.0, 0.0), (-2.0, 1.0, 0.0), (-3.0, 1.0, 0.0)];
		assert_eq![bgjk(&cube1, &cube2), false];
	}

	#[test]
	fn exact_overlap() {
		let cube1 = pts![(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (1.0, 1.0, 0.0)];
		let cube2 = pts![(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (1.0, 1.0, 0.0)];
		assert_eq![bgjk(&cube1, &cube2), true];
	}

	#[test]
	fn line_overlap() {
		let cube1 = pts![(0.0, 0.0, 0.0), (1.0, 0.0, 0.0)];
		let cube2 = pts![(0.5, 1.0, 0.0), (0.5, -1.0, 0.0)];
		assert_eq![bgjk(&cube1, &cube2), true];
	}

	#[test]
	fn point_overlap() {
		let cube1 = pts![(0.5, 1.0, 0.0)];
		let cube2 = pts![(0.5, 1.0, 0.0)];
		assert_eq![bgjk(&cube1, &cube2), true];
	}

	#[test]
	fn point_no_overlap() {
		let cube1 = pts![(0.5, 1.0, 0.0)];
		let cube2 = pts![(1.0, 1.0, 0.0)];
		assert_eq![bgjk(&cube1, &cube2), false];
	}

	#[test]
	fn empty_no_overlap() {
		// An empty set defaults to a single point in origo in the set
		let cube1: Vec<Vec3> = pts![];
		let cube2 = pts![(1.0, 1.0, 1.0)];
		assert_eq![bgjk(&cube1, &cube2), false];
	}

	#[test]
	fn side_by_side_squares() {
		let cube1 = pts![(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (1.0, 1.0, 0.0)];
		let cube2 = pts![(1.0, 0.0, 0.0), (2.0, 0.0, 0.0), (1.0, 1.0, 0.0), (2.0, 1.0, 0.0)];
		assert_eq![bgjk(&cube1, &cube2), true];
	}

	#[test]
	fn side_by_side_squares_offset() {
		let cube1 = pts![(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (1.0, 1.0, 0.0)];
		static TINY: f32 = f32::EPSILON;
		let cube2 =
			pts![(1.0 + TINY, 0.0, 0.0), (2.0, 0.0, 0.0), (1.0 + TINY, 1.0, 0.0), (2.0, 1.0, 0.0)];
		assert_eq![bgjk(&cube1, &cube2), false];
	}

	#[test]
	fn single_point_square_overlap() {
		let cube1 = pts![(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (1.0, 1.0, 0.0)];
		let cube2 = pts![(1.0, 1.0, 0.0), (2.0, 1.0, 0.0), (1.0, 2.0, 0.0), (2.0, 2.0, 0.0)];
		assert_eq![bgjk(&cube1, &cube2), true];
	}

}

fn main() {

	setup_logger();
	info!["Logger initialized"];

	let cube1 = vec![Vec3(0.0, 0.0, 0.0),
	                 Vec3(1.0, 0.0, 0.0),
	                 Vec3(0.0, 1.0, 0.0),
	                 Vec3(1.0, 1.0, 0.0),

	                 Vec3(2.0, 0.0, 1.0),
	                 Vec3(3.0, 0.0, 1.0),
	                 Vec3(2.0, 1.0, 1.0),
	                 Vec3(3.0, 1.0, 1.0)];

	let cube2 = vec![Vec3(-2.0, 0.0, 0.0),
	                 Vec3(-3.0, 0.0, 0.0),
	                 Vec3(-2.0, 1.0, 0.0),
	                 Vec3(-3.0, 1.0, 0.0),

	                 Vec3(0.0, 0.0, 1.0),
	                 Vec3(1.0, 0.0, 1.0),
	                 Vec3(0.0, 1.0, 1.0),
	                 Vec3(1.9, 1.0, 1.0)];

	use slog::{Serializer, Record};
	use slog::ser::Error;
	impl slog::Serialize for Vec3 {
		fn serialize(&self,
		             record: &Record,
		             key: &str,
		             serializer: &mut Serializer)
		             -> Result<(), Error> {
			serializer.emit_arguments(&key, &format_args!["{:?}", *self])
		}
	}
	info!["Tetra"; "value" => Vec3(0.0, 1.0, 2.0)];
	info!["Collision"; "status" => bgjk(&cube1, &cube2)];

	return;

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

		let side_speed = 0.4;
		let vert_speed = 0.45;
		if Key::A.is_pressed() {
			coller.enqueue(Vector(-side_speed, 0.0));
		}
		if Key::D.is_pressed() {
			coller.enqueue(Vector(side_speed, 0.0));
		}
		if Key::J.is_pressed() {
			coller2.enqueue(Vector(-side_speed, 0.0));
		}
		if Key::L.is_pressed() {
			coller2.enqueue(Vector(side_speed, 0.0));
		}

		rarer.run(|| println!("{:?}", coller));

		// This is a little messy. What can we do?
		// Try x movement.
		// If the movement has a collision:
		//   Move up 1
		//   If not possible, return
		//   Try to move x again
		//   If not possible, return
		let oldxspeed = coller.queued();
		let dy = coller.check_x();
		coller.solve(&net);
		coller.uncheck_x(dy);

		let oldpos = coller.get_pos();

		coller.set_speed(Vector(0.0, -1.001));
		if coller.tried_up {
			coller.solve(&net);
		}
		coller.set_speed(oldxspeed);

		let dy = coller.check_x();
		coller.solve(&net);
		coller.uncheck_x(dy);

		if coller.tried_up {
			coller.set_pos(oldpos);
		}

		let dy = coller2.check_x();
		coller2.solve(&net);
		coller2.uncheck_x(dy);

		if Key::W.is_pressed() && coller.jmp {
			coller.set_speed(Vector(0.0, -vert_speed));
			coller.jmp = false;
		}
		if Key::S.is_pressed() {
			coller.enqueue(Vector(0.0, vert_speed * 100000.0));
		}
		if Key::I.is_pressed() && coller2.jmp {
			coller2.set_speed(Vector(0.0, -vert_speed));
			coller2.jmp = false;
		}
		if Key::K.is_pressed() {
			coller2.enqueue(Vector(0.0, vert_speed * 100000.0));
		}

		coller.reset_dx();
		coller.enqueue(Vector(0.0, gravity));
		coller.solve(&net);
		coller2.enqueue(Vector(0.0, gravity));
		coller2.solve(&net);

		window.clear(&Color::new_rgb(255, 255, 255));

		let mut view = View::new_init(&Vector2f::new(0.0, 0.0), &Vector2f::new(800.0, 600.0))
			.unwrap();

		let pos = coller.get_pos();

		view.set_center(&Vector2f::new(pos.0 * 10.0, pos.1 * 10.0));
		window.set_view(&view);

		for i in net.view_center_f32((pos.0, pos.1), (41usize, 30usize)) {
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
	net.set_box(&0, (0, 0), (1000, 1000));
	net.set_box(&1, (1, 1), (999, 999));
	net.set_box(&0, (2, 2), (998, 998));
	net.set_box(&1, (1, 970), (20, 990));
	(0..20usize)
		.inspect(|x| {
			net.set(&1, (20 + x, 998 - x));
			net.set(&1, (21 + x, 998 - x));
		})
		.count();
	(0..20usize)
		.inspect(|x| {
			net.set(&1, (20 - x, 978 - x));
			net.set(&1, (21 - x, 978 - x));
		})
		.count();
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
				if let Key::Escape = code {
					return true;
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
	downward: bool,
}

impl RectsWhite {
	fn new() -> RectsWhite {
		RectsWhite {
			pts: vec![(0.001, 0.001), (1.0, 0.001), (0.001, 1.0), (1.0, 1.0)],
			pos: Vector(2.0, 970.0),
			mov: Vector(0.0, 0.0),
			jmp: false,
			checking_x: false,
			downward: false,
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

	fn enqueue(&mut self, vector: Vector) {
		self.mov = self.mov + vector;
	}
}

impl Collable<usize> for RectsWhite {
	fn presolve(&mut self) {
		if !self.checking_x {
			self.downward = self.mov.1 > 1e-6;
		}
	}

	fn postsolve(&mut self, collided_once: bool, _resolved: bool) {
		if !self.checking_x {
			if collided_once && self.downward {
				self.jmp = true;
			} else {
				self.jmp = false;
			}
		}
	}

	fn points(&self) -> Points {
		Points::new(self.pos, &self.pts)
	}

	fn queued(&self) -> Vector {
		self.mov
	}

	fn resolve<I>(&mut self, mut set: TileSet<usize, I>) -> bool
		where I: Iterator<Item = (i32, i32)>
	{
		let mut mov = self.mov;
		self.mov = Vector(0.0, 0.0);
		if set.all(|x| *x == 1) {
			self.pos = self.pos + mov;
			self.mov = Vector(0.0, mov.1);
			true
		} else if mov.norm2sq() > 1e-6 {
			if self.checking_x {
				mov = Vector(mov.0 * 0.5, mov.1);
				self.mov = mov;
			} else {
				let gravity = 0.00981;
				if mov.norm2sq() > gravity {
					self.mov = Vector(mov.0, -mov.1 * 0.7);
				} else {
					mov.scale(0.5);
					self.mov = mov;
				}
			}
			false
		} else {
			true
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
	downward: bool,
	tried_up: bool,
	collided_up: bool,
}

impl Rects {
	fn new() -> Rects {
		Rects {
			pts: vec![(0.001, 0.001), (1.0, 0.001), (0.001, 1.0), (1.0, 1.0)],
			pos: Vector(2.0, 990.0),
			mov: Vector(0.0, 0.0),
			jmp: false,
			checking_x: false,
			downward: false,
			tried_up: false,
			collided_up: false,
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

	fn reset_dx(&mut self) {
		self.mov = Vector(0.0, self.mov.1);
	}

	fn set_speed(&mut self, vec: Vector) {
		self.mov = vec;
	}

	fn get_pos(&self) -> Vector {
		self.pos
	}

	fn set_pos(&mut self, vec: Vector) {
		self.pos = vec;
	}

	fn enqueue(&mut self, vector: Vector) {
		self.mov = self.mov + vector;
	}
}

impl Collable<usize> for Rects {
	fn presolve(&mut self) {
		if !self.checking_x {
			self.downward = self.mov.1 > 1e-6;
			self.collided_up = false;
		} else {
			self.tried_up = false;
		}
	}

	fn postsolve(&mut self, collided_once: bool, _resolved: bool) {
		if !self.checking_x {
			self.collided_up = collided_once;
			if collided_once && self.downward {
				self.jmp = true;
			} else {
				self.jmp = false;
			}
		} else if collided_once {
			self.tried_up = true;
		}
	}

	fn points(&self) -> Points {
		Points::new(self.pos, &self.pts)
	}

	fn queued(&self) -> Vector {
		self.mov
	}

	fn resolve<I>(&mut self, mut set: TileSet<usize, I>) -> bool
		where I: Iterator<Item = (i32, i32)>
	{
		let mut mov = self.mov;
		self.mov = Vector(0.0, 0.0);
		if set.all(|x| *x == 0usize) {
			self.pos = self.pos + mov;
			self.mov = Vector(0.0, mov.1);
			true
		} else if mov.norm2sq() > 1e-6 {
			if self.checking_x {
				mov = Vector(mov.0 * 0.59, mov.1);
				self.mov = mov;
			} else {
				mov.scale(0.6);
				self.mov = mov;
			}
			false
		} else {
			true
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
