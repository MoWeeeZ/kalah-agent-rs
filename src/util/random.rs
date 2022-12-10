// Press, William H. / Teukolsky, Saul A. / Vetterling, William T. / Flannery, Brian P. (2007):
// Numerical Recipes 3rd Edition. The Art of Scientific Computing.
// Cambridge (Cambridge University Press).
// Pages 342-343
//
// original implementation in C++ ported to Rust

pub struct Rng {
    u: u64,
    v: u64,
    w: u64,
}

impl Rng {
    pub fn new(j: u64) -> Self {
        let v = 4101842887655102017;
        let w = 1;
        let u = j ^ v;

        let mut rng = Rng { u, v, w };
        rng.gen_u64();
        rng.v = rng.u;
        rng.gen_u64();
        rng.w = rng.v;
        rng.gen_u64();

        rng
    }

    pub fn gen_u64(&mut self) -> u64 {
        self.u = self.u * 2862933555777941757 + 7046029254386353087;

        self.v ^= self.v >> 17;
        self.v ^= self.v << 31;
        self.v ^= self.v >> 8;

        self.w = 4294957665 * (self.w & 0xffffffff) + (self.w >> 32);

        let mut x = self.u ^ (self.u << 21);
        x ^= x >> 35;
        x ^= x << 4;

        (x + self.v) ^ self.w
    }

    pub fn gen_u32(&mut self) -> u32 {
        self.gen_u64() as u32
    }

    pub fn gen_f64(&mut self) -> f64 {
        5.421_010_862_427_522e-20 * self.gen_u64() as f64
    }
}
