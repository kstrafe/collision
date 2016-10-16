use std::ops::{Neg, Sub};

#[derive(Clone, Copy, Debug, Default)]
pub struct Vec3(pub f32, pub f32, pub f32);

fn candidate(hull: &[Vec3]) -> Vec3 {
	if hull.len() == 0 {
		Vec3(0.0, 0.0, 0.0)
	} else {
		hull[0]
	}
}

/// The SSIA (Stravers' Simple Intersection Algorithm)
///
/// Takes two points on each hull. Uses the difference as a vector.
/// Finds a new optimal point for one hull, then the other.
/// If the new vector has a dot that's < 0 then there is intersection
pub fn ssia(hull1: &[Vec3], hull2: &[Vec3]) -> bool {
	println![""];
	let mut p = [Vec3::default(); 3];
	let mut q = [Vec3::default(); 3];
	p[0] = candidate(hull1);
	q[0] = candidate(hull2);
	let mut s = q[0] - p[0];

	loop {
		p[2] = p[1];
		p[1] = p[0];
		q[2] = q[1];
		q[1] = q[0];
		p[0] = farthest(p[0], hull1, s);
		q[0] = farthest(q[0], hull2, -s);
		s = q[0] - p[0];

		if s.norm2sq() == 0.0 {
			return true;
		}

		if p[0] == p[2] && q[0] == q[2] {
			println!["p: {:?}", p];
			println!["q: {:?}", q];
			let s2 = q[1] - p[0];

			let s3 = q[0] - p[1];
			let s4 = q[1] - p[1];

			let dir = p[1] - p[0];
			let dir2 = -dir;

			let dir3 = q[1] - q[0];
			let dir4 = -dir3;

			if (dir.dot(s) > 0.0 || dir.dot(s2) > 0.0) && (dir2.dot(s3) > 0.0 || dir2.dot(s4) > 0.0) {
				if (dir3.dot(-s) > 0.0 || dir3.dot(-s2) > 0.0) && (dir4.dot(-s3) > 0.0 || dir4.dot(-s4) > 0.0) {
					// Check if we can find an interpolation point, how do we do that?
					// If we can prove a plane exists between the lines: no collision
					// Problem: Farthest doesn't choose candidates based on angle from current position, we need that instead
					// The reason is that we sometimes alternate between disjoin lines which actually would collide
					// if we choose our farthest points correctly
					if cross(dir, dir3).dot(q[0] - p[0]).abs() < 0.0001 {
						return true;
					}
				}
				return false;
			} else {
				return false;
			}
		}
	}

	true
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
pub fn bgjk(hull1: &[Vec3], hull2: &[Vec3]) -> bool {
	let mut sp = Vec3::ones();
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

impl PartialEq for Vec3 {
	fn eq(&self, other: &Vec3) -> bool {
		self.0 == other.0 && self.1 == other.1 && self.2 == other.2
	}
}

impl Eq for Vec3 {}

impl Vec3 {
	fn dot(&self, right: Vec3) -> f32 {
		self.0 * right.0 + self.1 * right.1 + self.2 * right.2
	}

	fn ones() -> Vec3 {
		Vec3(1.0, 1.0, 1.0)
	}

	fn norm2sq(&self) -> f32 {
		self.0 * self.0 + self.1 * self.1 + self.2 * self.2
	}

	fn scale(&self, factor: f32) -> Vec3 {
		Vec3(self.0 * factor, self.1 * factor, self.2 * factor)
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


#[cfg(test)]
mod tests {

	use std::f32;
	use std::f32::consts::PI;
	use super::{Vec3, bgjk, ssia};
	static EPS: f32 = f32::EPSILON;

	macro_rules! pts {
		($($e:expr),*) => {
			[$(
				Vec3($e.0, $e.1, $e.2)
			),*]
		};
	}

	#[test]
	fn square1() {
		let cube1 = pts![(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (1.0, 1.0, 0.0)];
		let cube2 = pts![(-2.0, 0.0, 0.0), (-3.0, 0.0, 0.0), (-2.0, 1.0, 0.0), (-3.0, 1.0, 0.0)];
		assert_eq![bgjk(&cube1, &cube2), false];
		assert_eq![ssia(&cube1, &cube2), false];
	}

	#[test]
	fn exact_overlap() {
		let cube1 = pts![(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (1.0, 1.0, 0.0)];
		let cube2 = pts![(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (1.0, 1.0, 0.0)];
		assert_eq![bgjk(&cube1, &cube2), true];
		assert_eq![ssia(&cube1, &cube2), true];
	}

	#[test]
	fn line_overlap() {
		let cube1 = pts![(0.0, 0.0, 0.0), (1.0, 0.0, 0.0)];
		let cube2 = pts![(0.5, 1.0, 0.0), (0.5, -1.0, 0.0)];
		assert_eq![bgjk(&cube1, &cube2), true];
		assert_eq![ssia(&cube1, &cube2), true];
	}

	#[test]
	fn line_non_overlap() {
		let cube1 = pts![(0.0, 0.0, 0.0), (1.0, 0.0, 0.0)];
		let cube2 = pts![(1.5, 1.0, 0.0), (1.5, -1.0, 0.0)];
		assert_eq![bgjk(&cube1, &cube2), false];
		assert_eq![ssia(&cube1, &cube2), false];
	}

	#[test]
	fn small_line_point_overlap() {
		let cube1 = pts![(0.0, 0.0, 0.0), (0.01, 0.0, 0.0)];
		let cube2 = pts![(0.005, 0.0, 0.1)];
		assert_eq![bgjk(&cube1, &cube2), false];
		assert_eq![ssia(&cube1, &cube2), false];
	}

	#[test]
	fn line_point_non_overlap() {
		let cube1 = pts![(0.0, 0.0, 0.0), (1.0, 0.0, 0.0)];
		let cube2 = pts![(0.5, 0.0, 0.1)];
		assert_eq![bgjk(&cube1, &cube2), false];
		assert_eq![ssia(&cube1, &cube2), false];
	}

	#[test]
	fn point_overlap() {
		let cube1 = pts![(0.5, 1.0, 0.0)];
		let cube2 = pts![(0.5, 1.0, 0.0)];
		assert_eq![bgjk(&cube1, &cube2), true];
		assert_eq![ssia(&cube1, &cube2), true];
	}

	#[test]
	fn point_no_overlap() {
		let cube1 = pts![(0.5, 1.0, 0.0)];
		let cube2 = pts![(1.0, 1.0, 0.0)];
		assert_eq![bgjk(&cube1, &cube2), false];
		assert_eq![ssia(&cube1, &cube2), false];
	}

	#[test]
	fn empty_no_overlap() {
		// An empty set defaults to a single point in origo in the set
		let cube1: [Vec3; 0] = pts![];
		let cube2 = pts![(1.0, 1.0, 1.0)];
		assert_eq![bgjk(&cube1, &cube2), false];
		assert_eq![ssia(&cube1, &cube2), false];
	}

	#[test]
	fn side_by_side_squares() {
		let cube1 = pts![(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (1.0, 1.0, 0.0)];
		let cube2 = pts![(1.0, 0.0, 0.0), (2.0, 0.0, 0.0), (1.0, 1.0, 0.0), (2.0, 1.0, 0.0)];
		assert_eq![bgjk(&cube1, &cube2), true];
		assert_eq![ssia(&cube1, &cube2), true];
	}

	#[test]
	fn side_by_side_squares_offset() {
		let cube1 = pts![(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (1.0, 1.0, 0.0)];
		let cube2 =
			pts![(1.0 + EPS, 0.0, 0.0), (2.0, 0.0, 0.0), (1.0 + EPS, 1.0, 0.0), (2.0, 1.0, 0.0)];
		assert_eq![bgjk(&cube1, &cube2), false];
		assert_eq![ssia(&cube1, &cube2), false];
	}

	#[test]
	fn single_point_square_overlap() {
		let cube1 = pts![(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (1.0, 1.0, 0.0)];
		let cube2 = pts![(1.0, 1.0, 0.0), (2.0, 1.0, 0.0), (1.0, 2.0, 0.0), (2.0, 2.0, 0.0)];
		assert_eq![bgjk(&cube1, &cube2), true];
		assert_eq![ssia(&cube1, &cube2), true];
	}

	#[test]
	fn single_point_cube_overlap() {
		let cube1 = pts![(0.0, 0.0, 0.0),
		                 (1.0, 0.0, 0.0),
		                 (0.0, 1.0, 0.0),
		                 (1.0, 1.0, 0.0),
		                 (0.0, 0.0, 1.0),
		                 (1.0, 0.0, 1.0),
		                 (0.0, 1.0, 1.0),
		                 (1.0, 1.0, 1.0)];
		let cube2 = pts![(1.0, 1.0, 1.0),
		                 (2.0, 1.0, 1.0),
		                 (1.0, 2.0, 1.0),
		                 (2.0, 2.0, 1.0),
		                 (1.0, 1.0, 2.0),
		                 (2.0, 1.0, 2.0),
		                 (1.0, 2.0, 2.0),
		                 (2.0, 2.0, 2.0)];
		assert_eq![bgjk(&cube1, &cube2), true];
		assert_eq![ssia(&cube1, &cube2), true];
	}

	#[test]
	fn single_point_cube_non_overlap() {
		let cube1 = pts![(0.0, 0.0, 0.0),
		                 (1.0, 0.0, 0.0),
		                 (0.0, 1.0, 0.0),
		                 (1.0, 1.0, 0.0),
		                 (0.0, 0.0, 1.0),
		                 (1.0, 0.0, 1.0),
		                 (0.0, 1.0, 1.0),
		                 (1.0, 1.0, 1.0)];
		let cube2 = pts![(1.0, 1.0, 1.0 + EPS),
		                 (2.0, 1.0, 1.0 + EPS),
		                 (1.0, 2.0, 1.0 + EPS),
		                 (2.0, 2.0, 1.0 + EPS),
		                 (1.0, 1.0, 2.0),
		                 (2.0, 1.0, 2.0),
		                 (1.0, 2.0, 2.0),
		                 (2.0, 2.0, 2.0)];
		assert_eq![bgjk(&cube1, &cube2), false];
		assert_eq![ssia(&cube1, &cube2), false];
	}

	#[test]
	fn single_line_cube_overlap() {
		let cube1 = pts![(0.0, 0.0, 0.0),
		                 (1.0, 0.0, 0.0),
		                 (0.0, 1.0, 0.0),
		                 (1.0, 1.0, 0.0),
		                 (0.0, 0.0, 1.0),
		                 (1.0, 0.0, 1.0),
		                 (0.0, 1.0, 1.0),
		                 (1.0, 1.0, 1.0)];
		let cube2 = pts![(1.0, 1.0, 0.0),
		                 (2.0, 1.0, 0.0),
		                 (1.0, 2.0, 0.0),
		                 (2.0, 2.0, 0.0),
		                 (1.0, 1.0, 1.0),
		                 (2.0, 1.0, 1.0),
		                 (1.0, 2.0, 1.0),
		                 (2.0, 2.0, 1.0)];
		assert_eq![bgjk(&cube1, &cube2), true];
		assert_eq![ssia(&cube1, &cube2), true];
	}

	#[test]
	fn cube_projective_non_overlap() {
		let cube1 = pts![(0.0, 0.0, 0.0),
		                 (1.0, 0.0, 0.0),
		                 (0.0, 1.0, 0.0),
		                 (1.0, 1.0, 0.0),
		                 (1.0, 0.0, 1.0),
		                 (2.0, 0.0, 1.0),
		                 (1.0, 1.0, 1.0),
		                 (2.0, 1.0, 1.0)];
		let cube2 = pts![(1.1, 1.0, 0.0),
		                 (2.1, 1.0, 0.0),
		                 (1.1, 2.0, 0.0),
		                 (2.1, 2.0, 0.0),
		                 (2.1, 1.0, 1.0),
		                 (3.1, 1.0, 1.0),
		                 (2.1, 2.0, 1.0),
		                 (3.1, 2.0, 1.0)];
		assert_eq![bgjk(&cube1, &cube2), false];
		assert_eq![ssia(&cube1, &cube2), false];
	}

	#[test]
	fn cube_projective_overlap() {
		let cube1 = pts![(0.0, 0.0, 0.0),
		                 (1.0, 0.0, 0.0),
		                 (0.0, 1.0, 0.0),
		                 (1.0, 1.0, 0.0),
		                 (1.0, 0.0, 1.0),
		                 (2.0, 0.0, 1.0),
		                 (1.0, 1.0, 1.0),
		                 (2.0, 1.0, 1.0)];
		let cube2 = pts![(1.1, 1.0, 0.0),
		                 (2.1, 1.0, 0.0),
		                 (1.1, 2.0, 0.0),
		                 (2.1, 2.0, 0.0),
		                 (2.0, 1.0, 1.0),
		                 (3.1, 1.0, 1.0),
		                 (2.0, 2.0, 1.0),
		                 (3.1, 2.0, 1.0)];
		assert_eq![bgjk(&cube1, &cube2), true];
		assert_eq![ssia(&cube1, &cube2), true];
	}

	#[test]
	fn circle_non_overlap() {
		let (mut circle1, mut circle2) = (vec![], vec![]);
		let units = 100;
		circle1.reserve(units);
		circle2.reserve(units);
		for i in 0..units {
			let radian = i as f32 / units as f32 * 2.0 * PI;
			circle1.push(Vec3(radian.cos(), radian.sin(), 0.0));
			circle2.push(Vec3(radian.cos(), radian.sin(), EPS));
		}
		assert_eq![bgjk(&circle1, &circle2), false];
		ssia(&circle1, &circle2);
	}

	#[test]
	fn circle_overlap() {
		let (mut circle1, mut circle2) = (vec![], vec![]);
		let units = 100;
		circle1.reserve(units);
		circle2.reserve(units);
		for i in 0..units {
			let radian = i as f32 / units as f32 * 2.0 * PI;
			circle1.push(Vec3(radian.cos(), radian.sin(), 0.0));
			circle2.push(Vec3(radian.cos(), radian.sin(), 0.0));
		}
		assert_eq![bgjk(&circle1, &circle2), true];
		ssia(&circle1, &circle2);
	}

	#[test]
	fn circle_section() {
		let (mut circle1, mut circle2) = (vec![], vec![]);
		let units = 100;
		circle1.reserve(units);
		circle2.reserve(units);
		for i in 0..units {
			let radian = i as f32 / units as f32 * 2.0 * PI;
			circle1.push(Vec3(radian.cos(), radian.sin(), 0.0));
			circle2.push(Vec3(radian.cos() + 0.5, radian.sin(), 0.0));
		}
		assert_eq![bgjk(&circle1, &circle2), true];
		ssia(&circle1, &circle2);
	}

	#[test]
	fn circle_away() {
		let (mut circle1, mut circle2) = (vec![], vec![]);
		let units = 100;
		circle1.reserve(units);
		circle2.reserve(units);
		for i in 0..units {
			let radian = i as f32 / units as f32 * 2.0 * PI;
			circle1.push(Vec3(radian.cos(), radian.sin(), 0.0));
			circle2.push(Vec3(radian.cos() + 2.0 + 2.0 * EPS, radian.sin(), 0.0));
		}
		assert_eq![bgjk(&circle1, &circle2), false];
		ssia(&circle1, &circle2);
	}

}
