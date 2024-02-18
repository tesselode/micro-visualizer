use derive_more::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Rem, RemAssign, Sub, SubAssign};

#[derive(
	Debug,
	Clone,
	Copy,
	PartialEq,
	Eq,
	Hash,
	PartialOrd,
	Ord,
	Default,
	Add,
	AddAssign,
	Sub,
	SubAssign,
	Mul,
	MulAssign,
	Div,
	DivAssign,
	Rem,
	RemAssign,
)]
pub struct Frames(pub u64);

impl Frames {
	pub fn to_seconds(self, frame_rate: u64) -> Seconds {
		Seconds(self.0 as f64 / frame_rate as f64)
	}
}

#[derive(
	Debug,
	Clone,
	Copy,
	PartialEq,
	PartialOrd,
	Default,
	Add,
	AddAssign,
	Sub,
	SubAssign,
	Mul,
	MulAssign,
	Div,
	DivAssign,
	Rem,
	RemAssign,
)]
pub struct Seconds(pub f64);

impl Seconds {
	pub fn to_frames(self, frame_rate: u64) -> Frames {
		Frames((self.0 * frame_rate as f64) as u64)
	}

	pub fn abs(self) -> Seconds {
		Seconds(self.0.abs())
	}
}
