pub mod c;
pub mod m;
pub mod p;
pub mod s;
pub const N_ROUNDS_PS: [usize; 16] = [
    56, 57, 56, 60, 60, 63, 64, 63, 60, 66, 60, 65, 70, 60, 64, 68,
];
pub const N_ROUNDS_F: usize = 8;