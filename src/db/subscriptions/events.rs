bitflags::bitflags! {
    pub struct Events: i32 {
        const NEW = 0b0001;
        const UPD = 0b0010;
        const ALL = Self::NEW.bits | Self::UPD.bits;
    }
}

impl Default for Events {
    fn default() -> Self {
        Self::ALL
    }
}
