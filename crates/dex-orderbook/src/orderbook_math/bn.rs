use tonic_sdk_dex_types::U256;

pub const BPS_DIVISOR: u128 = 10_000;

#[macro_export]
macro_rules! BN {
    ($v:expr) => {
        BN(U256::from($v))
    };
}

pub struct BN(pub U256);

impl BN {
    pub fn mul(&self, v: u128) -> BN {
        BN(self.0 * U256::from(v))
    }

    pub fn div(&self, v: u128) -> BN {
        BN(self.0 / U256::from(v))
    }

    pub fn add(&self, v: u128) -> BN {
        BN(self.0 + U256::from(v))
    }

    pub fn sub(&self, v: u128) -> BN {
        BN(self.0 - U256::from(v))
    }

    pub fn as_u128(&self) -> u128 {
        self.0.as_u128()
    }

    pub fn as_u64(&self) -> u64 {
        self.0.as_u64()
    }

    pub fn add_bps(&self, bps: u16) -> BN {
        BN(self.0).mul(BPS_DIVISOR + bps as u128).div(BPS_DIVISOR)
    }

    pub fn sub_bps(&self, bps: u16) -> BN {
        BN(self.0).mul(BPS_DIVISOR - bps as u128).div(BPS_DIVISOR)
    }
}
